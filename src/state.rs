use crate::prelude::*;
use std::{convert::Infallible, sync::Arc};
use tokio::sync::mpsc::Sender;
use warp::{
    filters::{self, BoxedFilter},
    Filter,
};

pub struct State {
    pub db: DbOptions,
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

pub fn add_default(state: Arc<State>) -> BoxedFilter<(Arc<State>, String, String)> {
    warp::filters::any::any()
        .map(move || state.clone())
        .and(filters::header::header::<String>("Name"))
        .and(filters::header::header::<String>("Token"))
        .boxed()
}
