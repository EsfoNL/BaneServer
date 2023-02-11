use crate::cli::Cli;
use crate::prelude::*;
use sqlx::{mysql::MySqlConnectOptions, Connection, Executor, MySqlConnection, QueryBuilder, Row};
use warp::{http, Reply};

pub async fn configure(args: &Cli) -> Db {
    let options =
        MySqlConnectOptions::new().host(args.sqlserver.as_ref().map_or("127.0.0.1", |e| &e));
    Db::connect(options).await.unwrap()
}

pub async fn check_credentials(state: Arc<State>, id: &String, token: &String) -> http::StatusCode {
    let con = Db::connect_with(&state.db);
    if let Ok(mut con) = con.await {
        let query = con
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
    } else {
        eprintln!("cannot connect to database");
        return http::StatusCode::INTERNAL_SERVER_ERROR;
    }
}
