use serde::{Serialize, Deserialize};

use super::client::Ping;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Authenticate { token: String },
    BeginTyping { channel: String },
    EndTyping { channel: String },
    Subscribe { server_id: String },
    BeginEditing { channel: String, message: String },
    StopEditing { channel: String, message: String },
    Ping { data: Ping, responded: Option<()> },
}
