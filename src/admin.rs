use std::{
    convert::Infallible,
    str::FromStr,
    sync::Arc,
    time::{Duration, UNIX_EPOCH},
};

use crate::{api::filestream::file_uploader, prelude::State};
use argon2::{Argon2, PasswordVerifier, password_hash::phc::PasswordHash};
use axum::{
    Form, Router,
    extract::{FromRequestParts, OptionalFromRequestParts, Path, WebSocketUpgrade},
    response::{Html, Response},
    routing::{get, post},
};
use bane_server::{StrEnum, hash_password};
use base64::Engine;
use http::{StatusCode, request};
use rand::{RngExt, rngs::StdRng};
use serde::{Deserialize, Serialize};
use sqlx::query;
use ssq::{players::Player, rules::arma3::Arma3Rules};
use tera::Context;
use tracing::info;

type AState = axum::extract::State<Arc<State>>;

pub struct AuthUser {
    name: String,
}

impl FromRequestParts<Arc<State>> for AuthUser {
    type Rejection = Response;
    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &Arc<State>,
    ) -> Result<Self, Self::Rejection> {
        let redirect = || {
            Response::builder()
                .status(http::StatusCode::UNAUTHORIZED)
                .body(include_str!("../assets/redirect.html").into())
                .unwrap()
        };
        let cookie = parts
            .headers
            .get(http::header::COOKIE)
            .ok_or_else(redirect)?;
        let cookie = cookie.to_str().map_err(|_| redirect())?;
        let token = cookie.split("=").nth(1).ok_or_else(redirect)?;

        let row = sqlx::query!("select * from tokens where token = ?", token)
            .fetch_one(&state.db)
            .await
            .map_err(|_| redirect())?;
        if std::time::UNIX_EPOCH + Duration::from_secs(row.expires as u64)
            < std::time::SystemTime::now()
        {
            info!("token expired!");
            let _ = query!("delete from tokens where token = ?", token)
                .execute(&state.db)
                .await;
            return Err(redirect());
        }

        Ok(AuthUser { name: row.name })
    }
}

impl OptionalFromRequestParts<Arc<State>> for AuthUser {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &Arc<State>,
    ) -> Result<Option<Self>, Self::Rejection> {
        Ok(
            <Self as FromRequestParts<_>>::from_request_parts(parts, state)
                .await
                .ok(),
        )
    }
}

async fn login(state: AState, auth: Option<AuthUser>) -> Response {
    if auth.is_some() {
        return Response::builder()
            .status(http::StatusCode::SEE_OTHER)
            .header("Location", "/admin")
            .body(().into())
            .unwrap();
    }
    Response::new(state.pages.render("login", &Context::new()).unwrap().into())
}

#[derive(Deserialize)]
struct LoginData {
    name: String,
    password: String,
}

struct Row {
    name: String,
    hash: String,
    salt: String,
}

const EXPIRY_MINS: u64 = 20;
async fn login_post(
    state: AState,
    Form(data): Form<LoginData>,
) -> Result<Response, http::StatusCode> {
    // HeaderMap::new()
    let name = data.name;
    let Row { name, hash, salt } = sqlx::query_as!(
        Row,
        "select name, hash, salt from users where name = ?",
        name
    )
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // let decoded = PasswordHash::new(&salt).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    // let salt_bytes: Vec<u8> = base64::engine::general_purpose::STANDARD_PAD_INDIFFERENT
    //     .decode(salt)
    //     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if Argon2::default()
        .verify_password(
            data.password.as_bytes(),
            &PasswordHash::new(&hash).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        )
        .inspect_err(|e| {
            dbg!(e);
        })
        .is_ok()
    {
        let token = rand::make_rng::<StdRng>().random::<u128>().to_string();
        let expires = (std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            + Duration::from_mins(EXPIRY_MINS))
        .as_secs() as i64;
        sqlx::query!(
            "insert into tokens (name, token, expires) values (?, ?, ?)",
            name,
            token,
            expires,
        )
        .execute(&state.db)
        .await
        .unwrap();

        Ok(Response::builder()
            .header(
                http::header::SET_COOKIE,
                format!(
                    "token={token}; Max-Age={}; SameSite=Strict",
                    EXPIRY_MINS * 60
                ),
            )
            .header(http::header::LOCATION, "/admin")
            .status(http::StatusCode::SEE_OTHER)
            .body(().into())
            .unwrap())
        // todo!()
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
    // Argon2::default().hash_password_into_with_memory(pwd, salt, out, memory_blocks)
}

#[tracing::instrument(skip(state))]
async fn mission_uploaded(path: std::path::PathBuf, state: Arc<State>) {
    let mut arma_loc = state.args.arma_mission_dir.clone();
    arma_loc.push(path.file_name().unwrap());
    std::fs::copy(&path, &arma_loc).expect("failed to copy file");
    std::fs::remove_file(path).expect("failed to remove old file");
    // debug!("mission upload")
}

#[tracing::instrument(skip(state))]
async fn modpack_uploaded(path: std::path::PathBuf, state: Arc<State>) {
    let _ = tokio::process::Command::new(&state.args.modpack_install_script)
        .arg(path)
        .status()
        .await;
    // debug!("modpack upload")
}

pub struct Client<'a>(pub &'a State);
impl<'a> Client<'a> {
    async fn players(&self) -> ssq::errors::Result<Vec<Player>> {
        self.0.client.players(self.0.args.steam_query_addr).await
    }
    async fn arma3_rules(&self) -> ssq::errors::Result<Arma3Rules> {
        // info!("trying to get rules");
        let rules = self.0.client.rules(self.0.args.steam_query_addr).await?;
        // info!("rules: {rules:#?}");
        // info!("got rules");
        let arma_rules = Arma3Rules::from_rules(&rules)?;
        Ok(arma_rules)
    }
}

async fn admin(state: AState, _auth: AuthUser) -> Html<String> {
    let mut context = Context::new();
    let client = state.client();
    context.insert("players", &client.players().await.unwrap_or_default());
    info!("rules");
    let rules_result = client.arma3_rules().await;
    if let Ok(ref rules) = rules_result {
        context.insert("arma_rules", rules);
        // info!("inserted")
    } else {
        context.insert("arma_rules", &tera::Value::none());
    }
    // info!("rules_result: {rules_result:?}");
    state.pages.render("admin", &context).unwrap().into()
}
StrEnum!(Action, [Stop => "stop", Start => "start", Restart => "restart"]);

/// non validated action!
async fn arma_action(action: Action) -> Result<(), http::StatusCode> {
    tokio::process::Command::new("systemctl")
        .args([action.to_str(), "arma"])
        .status()
        .await
        .map(|_| ())
        .map_err(|_| http::StatusCode::INTERNAL_SERVER_ERROR)
}

async fn action(Path(action): Path<String>, _auth: AuthUser) -> Result<(), http::StatusCode> {
    arma_action(Action::from_str(&action).map_err(|_| http::StatusCode::BAD_REQUEST)?).await
}

#[derive(Serialize)]
struct JsonPlayer {
    name: String,
}
impl TryFrom<Player> for JsonPlayer {
    type Error = ();

    fn try_from(value: Player) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name.try_into().map_err(|_| ())?,
        })
    }
}

async fn live_players(_auth: AuthUser, state: AState, upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(async move |mut conn| {
        loop {
            let Ok(_) = conn
                .send(axum::extract::ws::Message::Text(
                    serde_json::to_string(
                        &state
                            .client()
                            .players()
                            .await
                            .unwrap_or_default()
                            .into_iter()
                            .filter_map(|e| JsonPlayer::try_from(e).ok())
                            .collect::<Vec<_>>(),
                    )
                    .unwrap_or_default()
                    .into(),
                ))
                .await
            else {
                return;
            };
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    })
}

async fn profiles(state: &State) -> Result<Vec<String>, std::io::Error> {
    let mut iter = tokio::fs::read_dir(&state.args.arma_modprofiles_dir).await?;
    let mut res = vec![];

    while let Some(entry) = iter.next_entry().await? {
        res.push(entry.file_name().to_string_lossy().to_string());
    }

    Ok(res)
}

async fn switch_profile(
    _auth: AuthUser,
    Path(profile): Path<String>,
    state: AState,
) -> Result<(), http::StatusCode> {
    let int_err = |_| StatusCode::INTERNAL_SERVER_ERROR;
    let profiles = profiles(&state).await.map_err(int_err)?;

    if !profiles.contains(&profile) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let _ = arma_action(Action::Stop).await; // probably fine if it fails
    let _ = tokio::fs::remove_dir(&state.args.arma_profile_folder_name).await;
    let mut result_path = state.args.arma_modprofiles_dir.clone();
    result_path.push(profile);
    tokio::fs::symlink(result_path, &state.args.arma_profile_folder_name)
        .await
        .map_err(int_err)
}

pub fn admin_router() -> Router<Arc<State>> {
    Router::new()
        .route("/login", get(login))
        .route("/login", post(login_post))
        .route("/", get(admin))
        .route("/action/{*act}", post(action))
        .route("/switch-profile/{*profile}", post(switch_profile))
        .route(
            "/script/websocket/{*path}",
            get(|/* ensures user auth */ _auth: AuthUser, p, q, s, ws| {
                crate::script::websocket_scripts(std::path::Path::new("scripts/admin"), p, q, s, ws)
            }),
        )
        .route("/live-players", get(live_players))
        .nest("/modpack", file_uploader("/tmp", modpack_uploaded))
        .nest("/mission", file_uploader("/tmp", mission_uploaded))
}
