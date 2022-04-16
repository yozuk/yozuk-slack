use anyhow::Result;
use clap::Parser;
use futures_util::StreamExt;
use lazy_regex::regex_replace_all;
use mediatype::MediaTypeBuf;
use reqwest::header;
use std::convert::Infallible;
use std::net::SocketAddrV4;
use std::str::FromStr;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;
use warp::Filter;
use yozuk::{ModelSet, Yozuk, YozukError};
use yozuk_sdk::prelude::*;

mod args;
mod block;
mod event;
mod message;

use args::*;
use block::*;
use event::*;
use message::*;

const API_URL_AUTH_TEST: &str = "https://slack.com/api/auth.test";
const API_URL_POST_MESSAGE: &str = "https://slack.com/api/chat.postMessage";
const API_URL_VIEWS_PUBLISH: &str = "https://slack.com/api/views.publish";

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::try_parse()?;

    let mut headers = header::HeaderMap::new();
    let mut auth_value = header::HeaderValue::from_str(&format!("Bearer {}", args.token)).unwrap();
    auth_value.set_sensitive(true);
    headers.insert(header::AUTHORIZATION, auth_value);

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let model = ModelSet::from_data(yozuk_bundle::MODEL_DATA).unwrap();
    let yozuk = Arc::new(Yozuk::builder().build(model));

    let identity = client
        .post(API_URL_AUTH_TEST)
        .send()
        .await?
        .json::<Identity>()
        .await?;

    let route = warp::any().and(warp::body::json()).and_then(move |event| {
        handle_message(event, yozuk.clone(), client.clone(), identity.clone())
    });

    warp::serve(route)
        .run(SocketAddrV4::new(args.addr, args.port))
        .await;

    Ok(())
}

async fn handle_message(
    event: Event,
    zuk: Arc<Yozuk>,
    client: reqwest::Client,
    identity: Identity,
) -> Result<warp::reply::Json, Infallible> {
    match event {
        Event::EventCallback(cb) => match cb.event {
            MessageEvent::AppMention(msg) => {
                handle_request(msg, zuk, client).await.unwrap();
            }
            MessageEvent::Message(msg) => {
                if msg.user != identity.user_id {
                    handle_request(msg, zuk, client).await.unwrap();
                }
            }
            MessageEvent::AppHomeOpened(event) => {
                publish_home(client, event.user).await.unwrap();
            }
        },
        Event::UrlVerification(event) => return Ok(handle_url_verification(event)),
    }
    Ok(warp::reply::json(&"ok".to_string()))
}

async fn publish_home(client: reqwest::Client, user_id: String) -> Result<()> {
    client
        .post(API_URL_VIEWS_PUBLISH)
        .json(&ViewsPublish {
            user_id,
            view: View {
                ty: "home".into(),
                blocks: vec![Block {
                    ty: "section".into(),
                    text: Some(Text {
                        ty: "mrkdwn".into(),
                        text: "Hello, I'm Yozuk.".into(),
                    }),
                }],
            },
        })
        .send()
        .await?;
    Ok(())
}

async fn handle_request(msg: Message, zuk: Arc<Yozuk>, client: reqwest::Client) -> Result<()> {
    let text = regex_replace_all!(
        r#"<@\w+>"#i,
        & msg.text,
        |_| String::new(),
    );
    let text = regex_replace_all!(
        r#"<[^|]+\|([^>]+)>"#i,
        &text,
        |_, text| format!("{}", text),
    );
    let text = regex_replace_all!(
        r#"<([^>]+)>"#i,
        &text,
        |_, text| format!("{}", text),
    );
    let text = gh_emoji::Replacer::new().replace_all(&text);
    println!("{:?}", text);

    let mut streams = futures_util::future::try_join_all(msg.files.iter().map(file_stream)).await?;

    let tokens = Yozuk::parse_tokens(&text);
    let result = zuk
        .get_commands(&tokens, &streams)
        .and_then(|commands| zuk.run_commands(commands, &mut streams, &Default::default()));

    let output = match result {
        Ok(output) => output,
        Err(YozukError::UnintelligibleRequest { .. }) => {
            let massage = PostMessage {
                channel: msg.channel.clone(),
                text: Some("Sorry, I can't understand your request.".into()),
                ..Default::default()
            };
            client
                .post(API_URL_POST_MESSAGE)
                .json(&massage)
                .send()
                .await?;
            return Ok(());
        }
        Err(YozukError::CommandError { mut errors }) => errors.pop().unwrap(),
    };

    for section in output.sections {
        let massage = if section.kind == SectionKind::Comment {
            PostMessage {
                channel: msg.channel.clone(),
                text: Some(section.as_utf8().into()),
                ..Default::default()
            }
        } else {
            PostMessage {
                channel: msg.channel.clone(),
                blocks: Some(vec![Block {
                    ty: "section".into(),
                    text: Some(Text {
                        ty: "mrkdwn".into(),
                        text: section.as_utf8().into(),
                    }),
                }]),
                ..Default::default()
            }
        };
        client
            .post(API_URL_POST_MESSAGE)
            .json(&massage)
            .send()
            .await?;
    }

    Ok(())
}

fn handle_url_verification(verification: UrlVerification) -> warp::reply::Json {
    warp::reply::json(&UrlVerificationReply {
        challenge: verification.challenge,
    })
}

async fn file_stream(file: &File) -> anyhow::Result<InputStream> {
    let tmpfile = NamedTempFile::new()?;
    let filepath = tmpfile.into_temp_path();
    let mut tmpfile = tokio::fs::File::create(&filepath).await?;
    let mut stream = reqwest::get(&file.url_private_download)
        .await?
        .bytes_stream();
    while let Some(data) = stream.next().await {
        tmpfile.write(&data?).await?;
    }
    Ok(InputStream::new(
        std::fs::File::open(filepath)?,
        MediaTypeBuf::from_str(&file.mimetype).unwrap(),
    ))
}
