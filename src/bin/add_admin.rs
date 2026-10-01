use argon2::password_hash::generate_salt;
use base64::Engine;
use clap::Parser;
use sqlx::{ConnectOptions, query, sqlite::SqliteConnectOptions};

#[derive(clap::Parser)]
struct Cli {
    name: String,
    password: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let salt = generate_salt();
    let hash = bane_server::hash_password(&cli.password, &salt).unwrap();

    let mut db = SqliteConnectOptions::new()
        .filename("db.sqlite")
        .connect()
        .await
        .unwrap();

    let salt_str = base64::engine::general_purpose::STANDARD.encode(salt);
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
