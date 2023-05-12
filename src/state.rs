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
    pub tera: RwLock<Tera>,
    pub context: tera::Context,
    pub watcher: std::sync::Mutex<Option<Box<dyn notify::Watcher + Send + Sync>>>,
}

impl State {
    pub async fn new(args: Cli) -> Self {
        let db = crate::db::configure(&args).await;
        let subscribers = dashmap::DashMap::new();
        let mut tera = Tera::new("templates/**").unwrap();
        tera.register_function("command", crate::webpages::command);
        for i in tera.get_template_names() {
            eprintln!("template: {i}")
        }
        let context = tera::Context::new();

        State {
            db,
            args,
            subscribers,
            tera: RwLock::new(tera),
            context,
            watcher: None.into(),
        }
    }
}

pub fn add_default(state: Arc<State>) -> BoxedFilter<(Arc<State>,)> {
    warp::filters::any::any().map(move || state.clone()).boxed()
}
