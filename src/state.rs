use crate::{api::filestream, prelude::*};

use bane_server::DebugIgnore;
use futures::channel::mpsc::Sender;
use notify::INotifyWatcher;
use ssq::nonblocking::Client;
use std::{fmt::Debug, sync::LazyLock};
use tera::Tera;
use tokio::sync::RwLock;
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
    pub client: DebugIgnore<ssq::nonblocking::Client>,
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
        let client = DebugIgnore(Client::new().await.unwrap());

        State {
            db,
            subscribers,
            tera: RwLock::new(tera.ok()),
            context,
            watcher: RwLock::new(None),
            args,
            pages: LazyLock::new(Box::new(|| {
                let mut tera = Tera::new();
                tera.load_from_glob("pages/**");
                tera
            })),
            filestreams: Default::default(),
            client,
        }
    }
    pub fn client(&self) -> crate::admin::Client {
        crate::admin::Client(&self)
    }
}
