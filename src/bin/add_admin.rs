use clap::Parser;
use sqlx::{query, sqlite::SqliteConnectOptions, ConnectOptions};

#[derive(clap::Parser)]
struct Cli {
    name: String,
    password: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let salt = bane_server::salt();
    let hash = bane_server::hash_password(&cli.password, salt.as_salt());

    let mut db = SqliteConnectOptions::new()
        .filename("db.sqlite")
        .connect()
        .await
        .unwrap();

    let salt_str = salt.as_str();
    query!(
        "insert into users (name, hash, salt) values (?, ?, ?)",
        cli.name,
        hash,
        salt_str
    )
    .execute(&mut db)
    .await
    .expect("failed to run query");
}
