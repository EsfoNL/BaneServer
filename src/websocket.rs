use std::error::Error;

use crate::{db, prelude::*};
use futures::{SinkExt, StreamExt};
use warp::ws::Ws;
use warp::{http, ws::WebSocket};

pub async fn handler(websocket: Ws, state: Arc<State>, token: String) -> impl warp::Reply {
    // let con = Db::connect_with(&state.db).await;
    // if let Ok(db_con) = con {
    // let login_result = db::check_credentials(state.clone(), &id, &token).await;
    // if login_result == http::StatusCode::OK {
    let id = 0;
    return websocket.on_upgrade(move |ws| websocket_handler(ws, state, id));
}

async fn websocket_handler(mut ws: WebSocket, state: Arc<State>, id: Id) {
    println!("handler!");
    while let Some(value) = ws.next().await {
        ws.send(value.unwrap()).await.unwrap()
    }
}
