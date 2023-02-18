use crate::prelude::*;

use futures::{channel::mpsc::Sender, lock::Mutex};
use sqlx::{Executor, Row};
use std::sync::Arc;
use warp::{filters::BoxedFilter, Filter};

pub struct State {
    pub db: Db,
    pub args: Cli,
    pub subscribers: dashmap::DashMap<Id, Sender<RecvMessage>>,
    pub users: Mutex<rangemap::RangeInclusiveSet<Id>>,
}

impl State {
    pub async fn new(args: Cli) -> Self {
        let db = crate::db::configure(&args).await;
        let subscribers = dashmap::DashMap::new();
        let mut users = rangemap::RangeInclusiveSet::new();

        for i in db
            .fetch_all(sqlx::query!("select id from ACCOUNTS"))
            .await
            .unwrap()
            .into_iter()
            .map(|e| e.get::<Id, _>(0))
        {
            users.insert(i..=i);
        }
        State {
            db,
            args,
            subscribers,
            users: Mutex::new(users),
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
