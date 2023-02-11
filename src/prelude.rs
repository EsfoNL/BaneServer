pub use crate::{cli::Cli, message::Message, state::State};
pub use dashmap::DashMap;
use sqlx::{MySqlConnection, MySqlPool};
pub use std::sync::Arc;
pub type Id = i64;
pub type Db = MySqlPool;
