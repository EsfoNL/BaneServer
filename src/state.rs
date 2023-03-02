use crate::prelude::*;

use futures::channel::mpsc::Sender;
use sqlx::{Executor, Row};
use std::sync::Arc;
use tera::Tera;
use tokio::sync::RwLock;
use warp::{filters::BoxedFilter, Filter};

pub struct State {
    pub db: Db,
    pub args: Cli,
    pub subscribers: dashmap::DashMap<Id, Sender<RecvMessage>>,
    pub tera: RwLock<Option<Tera>>,
}

impl State {
    pub async fn new(args: Cli) -> Self {
        let db = crate::db::configure(&args).await;
        let subscribers = dashmap::DashMap::new();
        let tera = Tera::new("templates/**").map_or(None, |e| Some(e));
        State {
            db,
            args,
            subscribers,
            tera: RwLock::new(tera),
        }
    }
}

pub fn add_default(state: Arc<State>) -> BoxedFilter<(Arc<State>,)> {
    warp::filters::any::any().map(move || state.clone()).boxed()
}
