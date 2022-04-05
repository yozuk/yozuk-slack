use anyhow::Result;
use reqwest::header;
use std::convert::Infallible;
use std::env;
use warp::Filter;

mod event;
mod message;

use event::*;
use message::*;

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

    let route = warp::any()
        .and(warp::body::json())
        .and_then(move |event| handle_message(event, client.clone()));

    warp::serve(route).run(([127, 0, 0, 1], 8080)).await;

    Ok(())
}

async fn handle_message(
    event: Event,
    client: reqwest::Client,
) -> Result<warp::reply::Json, Infallible> {
    match event {
        Event::EventCallback(cb) => match cb.event {
            MessageEvent::AppMention(msg) => {
                client
                    .post("https://slack.com/api/chat.postMessage")
                    .json(&PostMessage {
                        channel: msg.channel,
                        text: Some("Hello".into()),
                        thread_ts: Some(msg.ts),
                        reply_broadcast: true,
                    })
                    .send()
                    .await
                    .unwrap();
            }
            MessageEvent::Message(_msg) => {}
        },
        Event::UrlVerification(event) => return Ok(handle_url_verification(event)),
    }
    Ok(warp::reply::json(&"ok".to_string()))
}

fn handle_url_verification(verification: UrlVerification) -> warp::reply::Json {
    warp::reply::json(&UrlVerificationReply {
        challenge: verification.challenge.clone(),
    })
}
