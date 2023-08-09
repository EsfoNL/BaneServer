use crate::prelude::*;

use futures::channel::mpsc::Sender;
use notify::INotifyWatcher;
use reqwest::Client;
use std::fmt::Debug;
use tera::Tera;
use tokio::sync::RwLock;
use tracing::{error, info};

#[derive(Debug)]
pub struct State {
    pub db: Db,
    pub args: Cli,
    pub reqwest_client: Client,
    pub subscribers: dashmap::DashMap<Id, Sender<Message>>,
    pub tera: RwLock<Option<Tera>>,
    pub context: tera::Context,
    pub watcher: RwLock<Option<INotifyWatcher>>,
}

impl State {
    pub async fn new(args: Cli) -> Self {
        let db = crate::db::configure(&args).await;
        let subscribers = dashmap::DashMap::new();
        let mut tera = Tera::new(&format!("{}/**", &args.template_dir));
        match tera {
            Ok(ref mut tera) => {
                tera.register_function("command", crate::webpages::command);
                tera.register_function("sh", crate::webpages::shell_command);
                info!(
                    "loaded terra templates: {:#?}",
                    tera.get_template_names().collect::<Vec<&str>>()
                );
            }
            Err(ref err) => {
                error!("terra error: {err}")
            }
        }
        let context = tera::Context::new();

        State {
            db,
            args,
            subscribers,
            tera: RwLock::new(tera.ok()),
            context,
            reqwest_client: Client::new(),
            watcher: RwLock::new(None),
        }
    }
}
