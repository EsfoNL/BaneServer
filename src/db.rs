use crate::cli::Cli;
use sqlx::{mysql::MySqlConnectOptions, MySqlConnection};

pub type DbOptions = MySqlConnectOptions;
pub type Db = MySqlConnection;

pub async fn configure(args: &Cli) -> DbOptions {
    let options =
        MySqlConnectOptions::new().host(args.sqlserver.as_ref().map_or("127.0.0.1", |e| &e));
    todo!()
}
