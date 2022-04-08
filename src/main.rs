use anyhow::Result;
use clap::Parser;
use reqwest::header;
use std::convert::Infallible;
use std::net::SocketAddrV4;
use std::sync::Arc;
use warp::Filter;
use yozuk::{ModelSet, Yozuk, YozukError};

mod args;
mod event;
mod message;

use args::*;
use event::*;
use message::*;

const API_URL_AUTH_TEST: &str = "https://slack.com/api/auth.test";
const API_URL_POST_MESSAGE: &str = "https://slack.com/api/chat.postMessage";

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
                handle_request(msg.text, msg.channel, zuk, client, identity)
                    .await
                    .unwrap();
            }
            MessageEvent::Message(msg) => {
                if msg.user != identity.user_id {
                    handle_request(msg.text, msg.channel, zuk, client, identity)
                        .await
                        .unwrap();
                }
            }
        },
        Event::UrlVerification(event) => return Ok(handle_url_verification(event)),
    }
    Ok(warp::reply::json(&"ok".to_string()))
}

async fn handle_request(
    text: String,
    channel: String,
    zuk: Arc<Yozuk>,
    client: reqwest::Client,
    identity: Identity,
) -> Result<()> {
    let mention = format!("<@{}>", identity.user_id);
    let text = text.replace(&mention, "");

    let tokens = Yozuk::parse_tokens(&text);
    let result = zuk
        .get_commands(&tokens, &[])
        .and_then(|commands| zuk.run_commands(commands, &mut []));

    let output = match result {
        Ok(output) => output,
        Err(YozukError::UnintelligibleRequest { .. }) => {
            return Ok(());
        }
        Err(YozukError::CommandError { mut errors }) => errors.pop().unwrap(),
    };

    for section in output.sections {
        client
            .post(API_URL_POST_MESSAGE)
            .json(&PostMessage {
                channel: channel.clone(),
                text: Some(section.as_utf8().into()),
                ..Default::default()
            })
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
