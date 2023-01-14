use sqlx::{Connection, Executor, QueryBuilder, Row};
use warp::ws::Ws;
use warp::{http, ws::WebSocket};

use crate::prelude::*;

pub async fn handler(
    websocket: Ws,
    state: Arc<State>,
    id: String,
    token: String,
) -> Result<(), http::StatusCode> {
    let con = Db::connect_with(&state.db);
    if 
}

async fn websocket_handler(websocket: WebSocket, id: String, state: Arc<State>) {}
