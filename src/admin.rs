use std::{
    convert::Infallible,
    path::PathBuf,
    sync::Arc,
    time::{Duration, UNIX_EPOCH},
};

use crate::{api::filestream::file_uploader, cli::Cli, prelude::State};
use argon2::password_hash::Salt;
use axum::{
    extract::{FromRequestParts, OptionalFromRequestParts, State as AState},
    response::{Html, Response},
    routing::{get, post},
    Form, Router,
};
use bane_server::hash_password;
use http::{request, StatusCode};
use rand::{rngs::StdRng, RngExt};
use serde::Deserialize;
use sqlx::query;
use tera::Context;
use tracing::{debug, info};

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
                .status(http::StatusCode::SEE_OTHER)
                .header(http::header::LOCATION, "/admin/login")
                .body(().into())
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
    return Response::new(state.pages.render("login", &Context::new()).unwrap().into());
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
) -> Result<Response, Response> {
    // HeaderMap::new()
    let name = data.name;
    let Row { name, hash, salt } = sqlx::query_as!(
        Row,
        "select name, hash, salt from users where name = ?",
        name
    )
    .fetch_one(&state.db)
    .await
    .map_err(|_| {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(().into())
            .unwrap()
    })?;

    let foreign_hash = hash_password(&data.password, Salt::new(&salt).unwrap());
    if hash == foreign_hash.to_string() {
        let token = rand::make_rng::<StdRng>().random::<u128>().to_string();
        let expires = (std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            + Duration::from_mins(EXPIRY_MINS))
        .as_secs() as i64;
        query!(
            "insert into tokens (name, token, expires) values (?, ?, ?)",
            name,
            token,
            expires,
        )
        .execute(&state.db)
        .await
        .unwrap();

        return Ok(Response::builder()
            .header(
                http::header::SET_COOKIE,
                format!("token={token}; Max-Age={}", EXPIRY_MINS * 60),
            )
            .header(http::header::LOCATION, "/admin")
            .status(http::StatusCode::SEE_OTHER)
            .body(().into())
            .unwrap());
        // todo!()
    } else {
        Err(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(().into())
            .unwrap())
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

async fn admin(AState(state): AState<Arc<State>>, auth: AuthUser) -> Html<String> {
    let mut context = Context::new();
    context.insert("name", &auth.name);
    state.pages.render("admin", &context).unwrap().into()
}

pub fn admin_router() -> Router<Arc<State>> {
    Router::new()
        .route("/login", get(login))
        .route("/login", post(login_post))
        .route("/", get(admin))
        .route(
            "/script/websocket/{*path}",
            get(|/* ensures user auth */ _auth: AuthUser, p, q, s, ws| {
                crate::script::websocket_scripts(std::path::Path::new("scripts/admin"), p, q, s, ws)
            }),
        )
        .nest("/modpack", file_uploader("/tmp".into(), modpack_uploaded))
        .nest("/mission", file_uploader("/tmp".into(), mission_uploaded))
}
