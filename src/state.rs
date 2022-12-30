use crate::{cli::Cli, db::DbOptions};
use std::{convert::Infallible, sync::{Arc, Mutex, mpsc::{channel, Receiver, Sender}}};
use warp::Filter;

pub struct State {
    pub db: DbOptions,
    pub args: Cli,
    pub subscribers: dashmap::DashMap<String, Sender>
}

impl State {
    pub async fn new(args: Cli) -> Self {
        State {
            db: crate::db::configure(&args).await,
            args,
            subscribers: Mutex<HashMap<String, >>;
        }
    }
}

pub fn add_state(
    state: Arc<State>,
) -> impl Filter<Extract = (Arc<State>,), Error = Infallible> + Clone {
    warp::filters::any::any().map(move || state.clone())
}
