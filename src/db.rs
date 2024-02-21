use crate::prelude::*;
use sqlx::mysql::MySqlConnectOptions;

pub async fn configure(args: &Cli) -> Db {
    let mut options: MySqlConnectOptions = MySqlConnectOptions::new()
        .port(args.sqlport)
        .host(&args.sqlhost);
    if let Some(ref username) = args.sqlusername {
        options = options.username(username);
    }
    if let Some(ref password) = args.sqlpassword {
        options = options.password(password);
    }
    Db::connect_lazy_with(options)
}
