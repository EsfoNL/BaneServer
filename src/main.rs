use std::sync::Arc;

use clap::Parser;

use warp::http::Response;
use warp::{filters, Filter};

mod cli;
mod db;
mod state;
mod websocket;
pub use db::{Db, DbOptions};
pub use state::State;

#[tokio::main]
async fn main() {
    let args = cli::Cli::parse();
    let state = Arc::new(State::new(args).await);
    let ok = warp::path("ok").map(|| warp::reply);

    // 404 page in case not available
    let page_404 = filters::any::any().map(|| {
        println!("ok!");
        Response::builder()
            .status(404)
            .body("<!DOCTYPE html><head><head/><body>404</body>")
    });

    let api_v0 = warp::path("api/v0/")
        .and(filters::ws::ws())
        .and(state::add_state(state.clone()))
        .and(filters::header::header::<String>("Name"))
        .and(filters::header::header::<String>("Token"))
        .and_then(websocket::handler)
        .recover(|e| format!("error: {}"));

    let addr = std::net::SocketAddr::new(
        std::net::Ipv4Addr::new(127, 0, 0, 1).into(),
        args.port.unwrap_or(String::from("80")).parse().unwrap(),
    );
    let req = ok.or(api_v0).or(page_404).map(|_| "error");

    warp::serve(req).run(addr).await;
}
