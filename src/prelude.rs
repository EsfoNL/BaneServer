pub use crate::{cli::Cli, state::State};
use sqlx::SqlitePool;
pub use std::sync::Arc;
pub type Id = u64;
pub type Db = SqlitePool;
pub use crate::message::MessageType;
pub use tracing::{debug, error, info, instrument, warn};
