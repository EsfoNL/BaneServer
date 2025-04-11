use crate::prelude::*;

use futures::channel::mpsc::Sender;
use notify::INotifyWatcher;
use std::fmt::Debug;
use tera::Tera;
use tokio::sync::RwLock;
#[derive(Debug)]
#[allow(unused)]
pub struct State {
    pub db: Db,
    pub args: Cli,
    pub subscribers: dashmap::DashMap<Id, Sender<MessageType>>,
    pub filestreams:
        dashmap::DashMap<uuid::Uuid, tokio::sync::oneshot::Sender<axum::extract::ws::WebSocket>>,
    pub tera: RwLock<Option<Tera>>,
    pub context: tera::Context,
    pub watcher: RwLock<Option<INotifyWatcher>>,
}

impl State {
    pub async fn new(args: Cli) -> Self {
        let db = crate::db::configure(&args).await;
        info!("db thing done");
        let subscribers = dashmap::DashMap::new();
        let filestreams = dashmap::DashMap::new();
        let tera = crate::webpages::tera(&args);
        let context = crate::webpages::tera_context(&args);
        if let Err(ref err) = tera {
            error!("Terra error: {err}");
        }

        State {
            db,
            subscribers,
            filestreams,
            tera: RwLock::new(tera.ok()),
            context,
            watcher: RwLock::new(None),
            args,
        }
    }
}
