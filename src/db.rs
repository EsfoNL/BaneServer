use crate::cli::Cli;
use crate::prelude::*;
use sqlx::{mysql::MySqlConnectOptions, ConnectOptions, Connection, Executor, QueryBuilder, Row};
use warp::http;

pub async fn configure(args: &Cli) -> Db {
    if args.dev {
        let options: MySqlConnectOptions = MySqlConnectOptions::new()
            .host(args.sqlserver.as_ref().map_or("127.0.0.1", |e| &e))
            .password(args.sqlpassword.as_ref().map_or("root", |e| e.as_str()))
            .username(args.sqlusername.as_ref().map_or("root", |e| e.as_str()))
            .database("db");
        Db::connect_with(options).await.unwrap()
    } else {
        Db::connect("mysql:///db?socket=/var/run/mysqld/mysqld.sock")
            .await
            .unwrap()
    }
}

pub async fn check_credentials(state: Arc<State>, id: &String, token: &String) -> http::StatusCode {
    let query = state
        .db
        .fetch_one(
            QueryBuilder::new("select id, expired from tokens where token = ?")
                .push_bind(token)
                .build(),
        )
        .await;
    if let Ok(e) = query {
        let db_id: String = e.get(0);
        if id == &db_id {
            return http::StatusCode::OK;
        } else {
            return http::StatusCode::UNAUTHORIZED;
        }
    } else {
        return http::StatusCode::UNAUTHORIZED;
    }
}
