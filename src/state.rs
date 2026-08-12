use crate::{api::filestream, prelude::*};

use dashmap::DashMap;
use futures::channel::mpsc::Sender;
use notify::INotifyWatcher;
use std::{fmt::Debug, sync::LazyLock};
use tera::Tera;
use tokio::sync::RwLock;
use uuid::Uuid;
#[derive(Debug)]
#[allow(unused)]
pub struct State {
    pub db: Db,
    pub args: Cli,
    pub filestreams: filestream::FileStreams,
    pub subscribers: dashmap::DashMap<Id, Sender<MessageType>>,
    pub tera: RwLock<Option<Tera>>,
    pub pages: LazyLock<Tera, Box<dyn Fn() -> Tera + Send>>,
    pub context: tera::Context,
    pub watcher: RwLock<Option<INotifyWatcher>>,
}

impl State {
    pub async fn new(args: Cli) -> Self {
        let db = crate::db::configure(&args).await;
        info!("db thing done");
        let subscribers = dashmap::DashMap::new();
        let tera = crate::webpages::tera(&args);
        let context = crate::webpages::tera_context(&args);
        if let Err(ref err) = tera {
            error!("Terra error: {err}");
        }

        State {
            db,
            subscribers,
            tera: RwLock::new(tera.ok()),
            context,
            watcher: RwLock::new(None),
            args,
            pages: LazyLock::new(Box::new(|| Tera::new("pages/**").unwrap())),
            filestreams: Default::default(),
        }
    }
}
