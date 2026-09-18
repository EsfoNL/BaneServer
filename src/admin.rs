use std::{
    convert::Infallible,
    sync::Arc,
    time::{Duration, UNIX_EPOCH},
};

use crate::{api::filestream::file_uploader, prelude::State};
use argon2::password_hash::Salt;
use axum::{
    extract::{
        FromRequestParts, OptionalFromRequestParts, Path, State as AState, WebSocketUpgrade,
    },
    response::{Html, Response},
    routing::{get, post},
    Form, Router,
};
use bane_server::hash_password;
use http::{request, StatusCode};
use rand::{rngs::StdRng, RngExt};
use serde::{Deserialize, Serialize};
use sqlx::query;
use ssq::{players::Player, rules::arma3::Arma3Rules};
use tera::Context;
use tracing::info;

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

async fn login(
    AState(state): axum::extract::State<Arc<State>>,
    auth: Option<AuthUser>,
) -> Response {
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
    AState(state): AState<Arc<State>>,
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

    let foreign_hash = hash_password(&data.password, Salt::new(&salt).unwrap())
        .map_err(|_| http::StatusCode::INTERNAL_SERVER_ERROR)?;
    if hash == foreign_hash {
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
        for (index, rule) in rules.iter().enumerate() {
            std::fs::write(
                format!("/home/ersa/Code/rust/ssq/rule-{index}-name"),
                rule.name.as_slice(),
            );
            std::fs::write(
                format!("/home/ersa/Code/rust/ssq/rule-{index}-value"),
                rule.value.as_slice(),
            );
        }
        let arma_rules = Arma3Rules::from_rules(&rules)?;
        Ok(arma_rules)
    }
}

async fn admin(AState(state): AState<Arc<State>>, _auth: AuthUser) -> Html<String> {
    let mut context = Context::new();
    let client = state.client();
    context.insert("players", &client.players().await.unwrap_or_default());
    info!("rules");
    let rules_result = client.arma3_rules().await;
    if let Ok(ref rules) = rules_result {
        context.insert("arma_rules", rules);
        info!("inserted")
    }
    // info!("rules_result: {rules_result:?}");
    state.pages.render("admin", &context).unwrap().into()
}

async fn action(Path(action): Path<String>, _auth: AuthUser) -> http::StatusCode {
    if ["start", "stop", "restart"].contains(&action.as_str()) {
        tokio::process::Command::new("systemctl")
            .args([action.as_str(), "arma"])
            .status()
            .await
            .map(|_| http::StatusCode::OK)
            .unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR)
    } else {
        http::StatusCode::BAD_REQUEST
    }
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

async fn live_players(
    _auth: AuthUser,
    AState(state): AState<Arc<State>>,
    upgrade: WebSocketUpgrade,
) -> Response {
    upgrade.on_upgrade(async move |mut conn| loop {
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
    })
}

pub fn admin_router() -> Router<Arc<State>> {
    Router::new()
        .route("/login", get(login))
        .route("/login", post(login_post))
        .route("/", get(admin))
        .route("/action/{*act}", post(action))
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
