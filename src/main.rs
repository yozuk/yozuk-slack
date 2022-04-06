use anyhow::Result;
use reqwest::header;
use std::convert::Infallible;
use std::env;
use warp::Filter;

mod event;
mod message;

use event::*;
use message::*;

const API_URL_AUTH_TEST: &str = "https://slack.com/api/auth.test";
const API_URL_POST_MESSAGE: &str = "https://slack.com/api/chat.postMessage";

#[tokio::main]
async fn main() -> Result<()> {
    let mut headers = header::HeaderMap::new();
    let mut auth_value =
        header::HeaderValue::from_str(&format!("Bearer {}", env::var("SLACK_TOKEN").unwrap()))
            .unwrap();
    auth_value.set_sensitive(true);
    headers.insert(header::AUTHORIZATION, auth_value);

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let identity = client
        .post(API_URL_AUTH_TEST)
        .send()
        .await?
        .json::<Identity>()
        .await?;

    let route = warp::any()
        .and(warp::body::json())
        .and_then(move |event| handle_message(event, client.clone(), identity.clone()));

    warp::serve(route).run(([127, 0, 0, 1], 8080)).await;

    Ok(())
}

async fn handle_message(
    event: Event,
    client: reqwest::Client,
    identity: Identity,
) -> Result<warp::reply::Json, Infallible> {
    match event {
        Event::EventCallback(cb) => match cb.event {
            MessageEvent::AppMention(msg) => {
                handle_request(msg.text, msg.channel, client, identity)
                    .await
                    .unwrap();
            }
            MessageEvent::Message(msg) => {
                if msg.user != identity.user_id {
                    handle_request(msg.text, msg.channel, client, identity)
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
    client: reqwest::Client,
    identity: Identity,
) -> Result<()> {
    let mention = format!("<@{}>", identity.user_id);
    let text = text.replace(&mention, "");
    client
        .post(API_URL_POST_MESSAGE)
        .json(&PostMessage {
            channel,
            text: Some(text),
            ..Default::default()
        })
        .send()
        .await?;
    Ok(())
}

fn handle_url_verification(verification: UrlVerification) -> warp::reply::Json {
    warp::reply::json(&UrlVerificationReply {
        challenge: verification.challenge.clone(),
    })
}
