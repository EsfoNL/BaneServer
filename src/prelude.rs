pub use crate::{cli::Cli, message::Message, state::State};
pub use dashmap::DashMap;
use sqlx::MySqlPool;
pub use std::sync::Arc;
pub type Id = u64;
pub type Db = MySqlPool;
pub const TOKEN_LENGTH: usize = 32;
pub const MAX_HASH_LENGTH: usize = argon2::Params::DEFAULT_OUTPUT_LEN;
pub const SALT_LENGTH: usize = 32;
