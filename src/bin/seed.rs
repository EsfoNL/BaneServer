use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, QueryBuilder, Sqlite};

#[tokio::main]
async fn main() {
    let options = SqliteConnectOptions::new()
        .filename("db.sqlite")
        .create_if_missing(true);
    let mut db = options.connect().await.unwrap();
    QueryBuilder::new("create table if not exists users (name text not null, hash text not null, salt text not null)").build().execute(&mut db).await.expect("failed to create table");
    QueryBuilder::new("create table if not exists tokens (name text not null, token text not null, expires integer not null)").build().execute(&mut db).await.expect("failed to create table");
}
