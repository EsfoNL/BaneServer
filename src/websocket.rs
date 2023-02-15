use crate::prelude::*;
use futures::{SinkExt, StreamExt};
use sqlx::Executor;
use warp::ws::WebSocket;
use warp::ws::Ws;
use warp::Reply;

pub async fn handler(websocket: Ws, state: Arc<State>, token: String, id: Id) -> impl warp::Reply {
    if state
        .db
        .fetch_one(sqlx::query!(
            "select hash, salt from ACCOUNTS where id = ?",
            id
        ))
        .await
        .is_err()
    {
        return warp::http::StatusCode::BAD_REQUEST.into_response();
    }

    return websocket
        .on_upgrade(move |ws| websocket_handler(ws, state, id))
        .into_response();
}

async fn websocket_handler(mut ws: WebSocket, _state: Arc<State>, _id: Id) {
    println!("handler!");
    while let Some(value) = ws.next().await {
        ws.send(value.unwrap()).await.unwrap()
    }
}
