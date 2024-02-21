use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageType {
    Message {
        target: Id,
        message: String,
    },
    File {
        target: Id,
        filename: String,
        size: u64,
    },
    FilePart {
        target: Id,
        filename: String,
        part: u64,
        hash: String,
        data: String,
    },
}
