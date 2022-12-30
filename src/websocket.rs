use sqlx::{query, query_builder, Connection, Executor, QueryBuilder};
use std::sync::Arc;
use warp::{http, ws::WebSocket};
use warp::ws::Ws;

use crate::{Db, DbOptions, State};

pub async fn handler(
    websocket: Ws,
    state: Arc<State>,
    id: String,
    token: String,
) -> Result<(), impl Into<http::Error>> {
    let con = Db::connect_with(&state.db);
    if let Ok(con) = con.await {
        let query = con
            .fetch_one(
                querybuilder::new("select id, expired from tokens where token = ?")
                    .push_bind(token)
                    .build(),
            )
            .await
        
        
    } else {
        eprintln!("cannot connect to database")
        Err(http::Error::from(500))
    }
}

async fn websocket_handler(websocket: WebSocket, name) {
    
}