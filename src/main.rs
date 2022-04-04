use warp::Filter;

mod event;
use event::*;

#[tokio::main]
async fn main() {
    let route = warp::any()
        .and(warp::body::json())
        .map(|event: Event| match event {
            Event::UrlVerification(event) => handle_url_verification(event),
            _ => warp::reply::json(&"ok".to_string()),
        });

    warp::serve(route).run(([127, 0, 0, 1], 8080)).await;
}

fn handle_url_verification(verification: UrlVerification) -> warp::reply::Json {
    warp::reply::json(&UrlVerificationReply {
        challenge: verification.challenge.clone(),
    })
}
