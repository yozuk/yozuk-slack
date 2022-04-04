use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    UrlVerification(UrlVerification),
}

#[derive(Debug, Deserialize)]
pub struct UrlVerification {
    pub token: String,
    pub challenge: String,
}

#[derive(Debug, Serialize)]
pub struct UrlVerificationReply {
    pub challenge: String,
}
