use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{ws::Message, Path, Query},
    response::IntoResponse,
};
use futures::SinkExt;
use tokio::io::AsyncReadExt;
use tracing::{debug, info};

use crate::{state::State, webpages::get_path_under_dir};

#[tracing::instrument(skip(state))]
pub async fn websocket_scripts(
    root: &std::path::Path,
    Path(path): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    axum::extract::State(state): axum::extract::State<Arc<State>>,
    ws: axum::extract::WebSocketUpgrade,
) -> axum::response::Response {
    info!("ws called: {path}");
    let Some(path) = get_path_under_dir(root, &path) else {
        return http::StatusCode::NOT_FOUND.into_response();
    };
    let Ok(query_json) = serde_json::to_string(&query) else {
        return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    info!("query worked");
    info!("script path: {path:?}");
    let mut proc = tokio::process::Command::new(path);
    proc.env("QUERY", query_json)
        .stdout(std::process::Stdio::piped());
    #[cfg(debug_assertions)]
    proc.env("DEV", "1");
    let Ok(mut child) = proc.spawn() else {
        return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    if child.stdout.is_none() {
        return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    info!("spawn worked!");

    ws.on_upgrade(|mut ws| async move {
        let mut out = child.stdout.take().unwrap();
        let mut buff = [0u8; 256];

        loop {
            tokio::select! {
                _ = ws.recv() => {
                    break;
                },
                data = out.read(&mut buff) => {
                    let Ok(l) = data else {
                        break;
                    };
                    if l == 0 { break; }
                    if ws.send(Message::Text(
                        String::from_utf8_lossy(&buff[..l]).to_string().into()
                    )).await.is_err() {
                        break;
                    };
                }

            }
        }

        debug!("aborted!");

        let _ = ws.close().await;
        debug!("websockets closed");
        let _ = child
            .kill()
            .await
            .inspect_err(|e| debug!("websocket error: {e:?}"));
        debug!("child ended")
    })
}
