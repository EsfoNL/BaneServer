use crate::prelude::*;
use sqlx::{query, sqlite::SqliteConnectOptions};

pub async fn configure(args: &Cli) -> Db {
    // let mut options: MySqlConnectOptions = MySqlConnectOptions::new()
    // .port(args.sqlport)
    // .host(&args.sqlhost);
    // if let Some(ref username) = args.sqlusername {
    //     options = options.username(username);
    // }
    // if let Some(ref password) = args.sqlpassword {
    //     options = options.password(password);
    // }
    let options = SqliteConnectOptions::new()
        .filename("db.sqlite")
        .create_if_missing(true);
    let db = Db::connect_with(options).await.unwrap();
    // sqlx::Sqlite::query("create table if not exists users (name text not null, hash text not null, salt text not null)").execute(&db);
    db
}
