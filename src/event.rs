use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    UrlVerification(UrlVerification),
    EventCallback(EventCallback),
}

#[derive(Debug, Deserialize)]
pub struct UrlVerification {
    pub token: String,
    pub challenge: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageEvent {
    AppMention(AppMention),
    Message(Message),
}

#[derive(Debug, Deserialize)]
pub struct EventCallback {
    pub event: MessageEvent,
}

#[derive(Debug, Deserialize)]
pub struct AppMention {
    pub channel: String,
    pub text: String,
    pub user: String,
    pub ts: String,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub channel: String,
    pub text: String,
    pub user: String,
    pub channel_type: String,
    pub ts: String,
}
