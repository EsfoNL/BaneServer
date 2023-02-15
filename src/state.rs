use crate::prelude::*;

use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use warp::{filters::BoxedFilter, Filter};

pub struct State {
    pub db: Db,
    pub args: Cli,
    pub subscribers: dashmap::DashMap<String, Sender<Message>>,
}

impl State {
    pub async fn new(args: Cli) -> Self {
        State {
            db: crate::db::configure(&args).await,
            args,
            subscribers: dashmap::DashMap::new(),
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
