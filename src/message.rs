use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum RecvMessage {
    Message { sender: Id, message: String },
    ImageMessage { sender: Id, url: String },
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum SendMessage {
    Message { message: String, reciever: Id },
    ImageMessage { sender: Id },
}
