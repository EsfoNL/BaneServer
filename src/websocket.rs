use crate::prelude::*;
use either::Either;
use futures::{Sink, Stream};
use futures::{SinkExt, StreamExt};
use sqlx::{Executor, Row};
use warp::ws::Ws;
use warp::ws::{Message, WebSocket};
use warp::Reply;

pub async fn handler(websocket: Ws, state: Arc<State>, token: String, id: Id) -> impl warp::Reply {
    if let Err(e) = crate::api::validate_token(&token, id, &state.db).await {
        return match e {
            crate::api::TokenError::Expired => warp::http::StatusCode::GONE,
            crate::api::TokenError::Else => warp::http::StatusCode::UNAUTHORIZED,
        }
        .into_response();
    }

    return websocket
        .on_upgrade(move |ws| websocket_handler(ws, state, id))
        .into_response();
}

async fn websocket_handler(ws: WebSocket, state: Arc<State>, id: Id) {
    let (sender, reciever) = futures::channel::mpsc::channel(CHANNEL_BOUND);
    state.subscribers.insert(id, sender.clone());
    let (mut ws_sender, ws_reciever) = ws.split();

    let mut combined_stream = futures::stream::select(
        ws_reciever.map(|e| Either::Left(e)),
        reciever.map(|e| Either::Right(e)),
    );

    while let Some(value) = combined_stream.next().await {
        match value {
            Either::Left(v) => {
                if v.is_err() {
                    state.subscribers.remove(&id);
                    combined_stream.into_inner().1.into_inner().close();
                    eprintln!("websocket closed {id}");
                    return ();
                }
                tokio::spawn(handle_request(v.unwrap(), id, state.clone()));
            }
            Either::Right(v) => {
                if ws_sender
                    .send(warp::ws::Message::text(serde_json::to_string(&v).unwrap()))
                    .await
                    .is_err()
                {
                    state.subscribers.remove(&id);
                    combined_stream.into_inner().1.into_inner().close();
                    match v {
                        RecvMessage::Message { sender, message } => {
                            store_message_db(message, sender, id, &state.db).await;
                        }
                        _ => (),
                    }
                    eprintln!("websocket closed {id}");
                    return ();
                }
            }
        }
    }
}

async fn handle_request(mesg: Message, id: Id, state: Arc<State>) {
    if let Ok(text) = mesg.to_str() {
        let parsed_message: SendMessage = serde_json::from_str(text).unwrap();
        match parsed_message {
            SendMessage::Message {
                message,
                receiver: reciever,
            } => {
                if let Some(mut conn) = state.subscribers.get_mut(&reciever) {
                    if conn.is_closed() {
                        store_message_db(message, id, reciever, &state.db).await;
                    } else {
                        eprintln!("sent through ws: {message}, {id}");
                        drop(
                            conn.send(RecvMessage::Message {
                                message,
                                sender: id,
                            })
                            .await,
                        );
                    }
                } else {
                    store_message_db(message, id, reciever, &state.db).await
                }
            }
        }
    }
}

async fn store_message_db(message: String, sender: Id, reciever: Id, db: &Db) {
    eprintln!("send to db {message}, {sender}, {reciever}");
    let mut trans = db.begin().await.unwrap();
    let queue_position_req = trans
        .fetch_optional(sqlx::query!(
            "select Max(queuepos) as pos from MESSAGES where sender = ? and reciever = ?",
            sender,
            reciever
        ))
        .await
        .unwrap();
    println!("{queue_position_req:?}");
    let queue_position =
        queue_position_req.map_or(0, |e| e.try_get::<u16, _>(0).map_or(0, |e| e + 1));
    trans
        .execute(sqlx::query!(
            "insert into MESSAGES (sender, reciever, message, queuepos) values (?, ?, ?, ?)",
            sender,
            reciever,
            message,
            queue_position
        ))
        .await
        .unwrap();
    trans.commit().await.unwrap();
}
