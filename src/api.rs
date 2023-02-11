use argon2::PasswordHasher;
use sqlx::{mysql::MySqlConnectOptions, Connection, Executor, MySqlConnection, Row};
use warp::Reply;

use crate::prelude::*;
use serde_json::{json, Serializer};

async fn poll_messages(state: State, id: Id, token: String) {}

pub async fn login(state: Arc<State>, email: String, password: String) -> impl Reply {
    let query = state
        .db
        .fetch_one(sqlx::query("select id, hash, salt from ACCOUNTS where email = ?").bind(email))
        .await;
    if let Ok(data) = query {
        let argon = argon2::Argon2::default();

        let hash = argon
            .hash_password(password.as_bytes(), data.get::<&str, _>("hash"))
            .map(|h| h.to_string());
        let db_hash: String = data.get("hash");
        if Ok(db_hash) == hash {
            if let Ok((token, refresh_token)) = generate_tokens(data.get("id"), &mut state.db).await
            {
                warp::http::Response::builder()
                    .status(200)
                    .body(json!({"token": token, "refresh_token": refresh_token,}).to_string())
                    .into_response()
            } else {
                warp::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        } else {
            warp::http::StatusCode::UNAUTHORIZED.into_response()
        }
    } else {
        warp::http::StatusCode::UNAUTHORIZED.into_response()
    }
}

/// (Token, RefreshToken)
async fn generate_tokens(id: Id, db: &mut MySqlConnection) -> Result<(String, String), ()> {
    db.execute(sqlx::query("delete from TOKENS where id = ?").bind(id));
    // con = Db::connect_with(db).await.map_err(|e| ())?;
    db.execute(sqlx::query("delete from REFRESH_TOKENS where id = ?").bind(id));
    todo!();
}
