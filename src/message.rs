use serde::{Deserialize, Serialize};

use crate::prelude::*;

const FILEPART_SIZE: u64 = 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
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
