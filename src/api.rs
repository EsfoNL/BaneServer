pub mod filestream;

use argon2::PasswordHasher;

use rand::distributions::{Alphanumeric, DistString};
use sqlx::{Executor, Row};

use crate::prelude::*;
use serde_json::json;

/*
pub enum TokenError {
    Expired,
    Else,
}

pub async fn poll_messages(state: Arc<State>, token: String, id: Id) -> impl Reply {
    if let Err(e) = validate_token(&token, id, &state).await {
        return match e {
            TokenError::Else => warp::http::StatusCode::UNAUTHORIZED,
            TokenError::Expired => warp::http::StatusCode::GONE,
        }
        .into_response();
    }
    let mut trans = state.db.begin().await.unwrap();
    let messages = trans
        .fetch_all(sqlx::query!(
            "select * from MESSAGES where reciever = ? order by queuepos",
            id
        ))
        .await;
    let res = messages.map_or(
        warp::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        |e| {
            let message_json = serde_json::Value::Array(
                e.into_iter()
                    .map(|e| {
                        let val = serde_json::to_value(Message::Message {
                            target: e.get("sender"),
                            message: e.get("message"),
                        })
                        .unwrap_or(serde_json::Value::Null);
                        eprintln!("{}", val.to_string());
                        val
                    })
                    .collect(),
            );
            warp::http::Response::builder()
                .status(200)
                .body(message_json.to_string())
                .into_response()
        },
    );

    trans
        .execute(sqlx::query!("delete from MESSAGES where reciever = ?", id))
        .await
        .unwrap();
    trans.commit().await.unwrap();
    res
}

pub async fn query_id(state: Arc<State>, id: Id) -> impl Reply {
    state
        .db
        .fetch_one(sqlx::query!(
            "select name, num from ACCOUNTS where id = ?",
            id
        ))
        .await
        .map_or(warp::http::StatusCode::NOT_FOUND.into_response(), |e| {
            warp::http::Response::builder()
                .status(200)
                .body(
                    json!({
                        "name": e.get::<String, _>("name"),
                        "num": e.get::<u16, _>("num")
                    })
                    .to_string(),
                )
                .map_or(warp::http::StatusCode::NOT_FOUND.into_response(), |e| {
                    e.into_response()
                })
        })
}

pub async fn query_name(state: Arc<State>, name: String) -> impl Reply {
    let mut split_name = name.split('#');
    let name = split_name.next().unwrap();
    let num: u16 = split_name.next().unwrap().parse().unwrap();
    return state
        .db
        .fetch_one(sqlx::query!(
            "select id from ACCOUNTS where name = ? and num = ?",
            name,
            num
        ))
        .await
        .map_or(warp::http::StatusCode::NOT_FOUND.into_response(), |e| {
            warp::http::Response::builder()
                .status(200)
                .body(e.get::<u64, _>(0).to_string())
                .map_or(warp::http::StatusCode::NOT_FOUND.into_response(), |e| {
                    e.into_response()
                })
        });
}

pub async fn login(state: Arc<State>, email: String, password: String) -> impl Reply {
    let query = state
        .db
        .fetch_one(sqlx::query!(
            "select id, hash, salt, name, num from ACCOUNTS where email = ?",
            email
        ))
        .await;
    if let Ok(data) = query {
        let hash = hash_data(&password, &data.get("salt"));
        let db_hash: String = data.get("hash");
        if db_hash == hash {
            if let Ok((token, refresh_token)) = generate_tokens(data.get("id"), &state.db).await {
                warp::http::Response::builder()
                    .status(200)
                    .body(
                        json!({
                            "token": token,
                            "refresh_token": refresh_token,
                            "id": data.get::<u64, _>("id"),
                            "name": data.get::<String, _>("name"),
                            "num": data.get::<u16, _>("num")
                        })
                        .to_string(),
                    )
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
async fn generate_tokens(id: Id, db: &Db) -> Result<(String, String), ()> {
    // remove old tokens
    db.execute(sqlx::query!("delete from TOKENS where id = ?", id))
        .await
        .unwrap();
    db.execute(sqlx::query!("delete from REFRESH_TOKENS where id = ?", id))
        .await
        .unwrap();

    let (token, token_salt) = generate_token_salt();
    let (refresh_token, refresh_token_salt) = generate_token_salt();

    // generate salts and hashes
    let token_hash = hash_data(&token, &token_salt);
    let refresh_token_hash = hash_data(&refresh_token, &refresh_token_salt);

    let token_expiry = time() + chrono::Days::new(7);
    let refresh_token_expiry = time() + chrono::Days::new(30);
    let mut transaction = db.begin().await.unwrap();
    let query1 = transaction
        .execute(sqlx::query!(
            "insert into REFRESH_TOKENS values (?, ?, ?, ?)",
            id,
            refresh_token_hash,
            refresh_token_salt,
            refresh_token_expiry
        ))
        .await;
    let query2 = transaction
        .execute(sqlx::query!(
            "insert into TOKENS values (?, ?, ?, ?)",
            id,
            token_hash,
            token_salt,
            token_expiry
        ))
        .await;
    if query1.is_ok() && query2.is_ok() {
        if transaction.commit().await.is_ok() {
            return Ok((token, refresh_token));
        } else {
            Err(())
        }
    } else {
        println!("token error: {:#?}, {:#?}!", query1, query2);
        Err(())
    }
}

fn generate_token_salt() -> (String, String) {
    let mut rng = rand::thread_rng();
    let token = Alphanumeric.sample_string(&mut rng, TOKEN_LENGTH);
    let token_salt = Alphanumeric.sample_string(&mut rng, SALT_LENGTH);
    (token, token_salt)
}

pub fn hash_data(password: &String, salt: &String) -> String {
    let argon = argon2::Argon2::default();
    let hash = argon
        .hash_password(password.as_bytes(), salt.as_str())
        .unwrap();
    let mut hash_string = [0u8; 86];
    hash.hash
        .unwrap()
        .b64_encode(&mut hash_string)
        .unwrap()
        .to_string()
}

fn generate_hash_password(password: &String) -> (String, String) {
    let mut rng = rand::thread_rng();
    let argon = argon2::Argon2::default();
    let salt = Alphanumeric.sample_string(&mut rng, SALT_LENGTH);
    let hash = argon
        .hash_password(password.as_bytes(), salt.as_str())
        .unwrap();
    println!("pw: {password}, hash: {:?}", hash.hash);
    let mut hash_string = [0u8; 86];
    (
        hash.hash
            .unwrap()
            .b64_encode(&mut hash_string)
            .unwrap()
            .to_string(),
        salt,
    )
}

pub async fn register(
    state: Arc<State>,
    email: String,
    password: String,
    name: String,
) -> impl Reply {
    println!("registration attempt!");
    let mut transaction = state.db.begin().await.unwrap();
    if transaction
        .fetch_optional(sqlx::query!(
            "select * from ACCOUNTS where email = ?",
            email
        ))
        .await
        .unwrap()
        .is_some()
    {
        return warp::http::Response::builder()
            .status(409)
            .body("there already exists an account with this email");
    }
    let (hash, salt) = generate_hash_password(&password);

    let num: u16 = (transaction
        .fetch_one(sqlx::query!(
            "select COUNT(num) as numcount from ACCOUNTS where name = ?",
            name
        ))
        .await
        .unwrap()
        .get::<i64, _>(0)
        + 1) as u16;
    let id: Id = transaction.fetch_one(sqlx::query!("select ab.id from ACCOUNTS as ab where not exists ( select * from ACCOUNTS as aa where aa.id = ab.id + 1 ) order by id asc limit 1;")).await.unwrap().get::<Id, _>(0) + 1;
    if num > 9999 {
        return warp::http::Response::builder()
            .status(409)
            .body("too many people with this username");
    }
    if let Err(e) = transaction
        .execute(sqlx::query!(
            "insert into ACCOUNTS values (?, ?, ?, ?, ?, ?)",
            id,
            email,
            hash,
            salt,
            name,
            num
        ))
        .await
    {
        println!("{:?}", e);
        return warp::http::Response::builder()
            .status(500)
            .body("creation error");
    }
    if transaction.commit().await.is_err() {
        println!("almost succesfull");
        return warp::http::Response::builder().status(500).body("");
    }
    warp::http::Response::builder()
        .status(200)
        .body("registration succesfull")
}

pub async fn validate_token(token: &String, id: Id, state: &Arc<State>) -> Result<(), TokenError> {
    if state.args.dev {
        return Ok(());
    }
    let optional_data = state
        .db
        .fetch_optional(sqlx::query!(
            "select token_hash, salt, token_expiry from TOKENS where id = ?",
            id
        ))
        .await
        .map_err(|_| TokenError::Else)?;
    if let Some(data) = optional_data {
        let hash = hash_data(token, &data.get("salt"));
        if hash != data.get::<String, _>("token_hash") {
            println!(
                "actual_token: {}, token: {}",
                data.get::<String, _>("token_hash"),
                hash
            );
            return Err(TokenError::Else);
        }
        if data.get::<Time, _>("token_expiry") > time() {
            Ok(())
        } else {
            state
                .db
                .execute(sqlx::query!("delete from TOKENS where id = ?", id))
                .await
                .unwrap();
            Err(TokenError::Expired)
        }
    } else {
        if state
            .db
            .fetch_optional(sqlx::query!(
                "select * from REFRESH_TOKENS where id = ?",
                id
            ))
            .await
            .map_err(|_| TokenError::Else)?
            .is_some()
        {
            Err(TokenError::Expired)
        } else {
            Err(TokenError::Else)
        }
    }
}

pub type Time = chrono::DateTime<chrono::Local>;
pub fn time() -> Time {
    chrono::Local::now()
}

pub fn add_token_id() -> BoxedFilter<(String, u64)> {
    warp::filters::any::any()
        .and(warp::header("token"))
        .and(warp::header("id"))
        .boxed()
}

pub async fn refresh_token(state: Arc<State>, id: Id, refresh_token: String) -> impl Reply {
    let mut trans = state.db.begin().await.unwrap();
    if let Ok(res) = trans
        .fetch_one(sqlx::query!(
            "select * from REFRESH_TOKENS where id = ?",
            id,
        ))
        .await
    {
        let hash = hash_data(&refresh_token, &res.get("salt"));
        eprintln!("{hash}, {}", res.get::<String, _>("refresh_token_hash"));
        if hash == res.get::<String, _>("refresh_token_hash") {
            trans
                .execute(sqlx::query!("delete from REFRESH_TOKENS where id = ?", id))
                .await
                .unwrap();
            trans
                .execute(sqlx::query!("delete from TOKENS where id = ?", id))
                .await
                .unwrap();
            trans.commit().await.unwrap();
            if res.get::<Time, _>("refresh_token_expiry") > time() {
                let (token, new_refresh_token) = generate_tokens(id, &state.db).await.unwrap();
                return warp::http::Response::builder()
                    .body(
                        json!({
                            "token": token,
                            "refresh_token": new_refresh_token
                        })
                        .to_string(),
                    )
                    .into_response();
            } else {
                return warp::http::StatusCode::GONE.into_response();
            }
        }
    }
    return warp::http::StatusCode::UNAUTHORIZED.into_response();
}
*/
