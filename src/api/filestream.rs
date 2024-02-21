use crate::prelude::*;
use axum::{
    extract::ws::*,
    response::{IntoResponse, Response},
};
use futures::StreamExt;
use serde::Deserialize;
use thiserror::Error;

#[instrument(skip(ws, state))]
pub async fn filestream_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<Arc<State>>,
) -> Response {
    let id = uuid::Uuid::new_v4();
    let (tx, rx) = tokio::sync::oneshot::channel();
    state.filestreams.insert(id, tx);
    ws.on_upgrade(move |ws| handler(ws, id, rx))
}

#[instrument(skip(ws, reciever))]
async fn handler(
    mut ws: WebSocket,
    id: uuid::Uuid,
    reciever: tokio::sync::oneshot::Receiver<WebSocket>,
) {
    info!("filestream connection established: {}", id);
    ws.send(Message::Text(id.to_string())).await;
    match reciever.await {
        Ok(v) => {
            use futures::{Stream, StreamExt};
            let (stx, srx) = ws.split();
            let (rtx, rrx) = v.split();
            tokio::spawn(srx.forward(rtx));
            tokio::spawn(rrx.forward(stx));
        }
        Err(v) => error!("{v}"),
    }
}

#[instrument(skip(ws, state))]
pub async fn filestream_reciever_handler(
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<Arc<State>>,
) -> Response {
    if !state.filestreams.contains_key(&id) {
        return http::StatusCode::NOT_FOUND.into_response();
    }
    info!("transfer started");

    ws.on_upgrade(move |ws| async move {
        if let Some(channel) = state.filestreams.remove(&id) {
            channel.1.send(ws);
        }
    })
}
