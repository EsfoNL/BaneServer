use crate::prelude::*;

use futures::{channel::mpsc::Sender, lock::Mutex};
use sqlx::{Executor, Row};
use std::sync::Arc;
use warp::{filters::BoxedFilter, Filter};

pub struct State {
    pub db: Db,
    pub args: Cli,
    pub subscribers: dashmap::DashMap<Id, Sender<RecvMessage>>,
}

impl State {
    pub async fn new(args: Cli) -> Self {
        let db = crate::db::configure(&args).await;
        let subscribers = dashmap::DashMap::new();
        State {
            db,
            args,
            subscribers,
        }
    }
}

pub fn add_token_id() -> BoxedFilter<(String, u64)> {
    warp::filters::any::any()
        .and(warp::header("Token"))
        .and(warp::header("Id"))
        .boxed()
}

pub fn add_default(state: Arc<State>) -> BoxedFilter<(Arc<State>,)> {
    warp::filters::any::any().map(move || state.clone()).boxed()
}
