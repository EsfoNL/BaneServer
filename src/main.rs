use clap::Parser;

use warp::http::Response;
use warp::{filters, Filter};

mod cli;
mod db;
mod prelude;
mod state;
mod websocket;
use prelude::*;

#[tokio::main]
async fn main() {
    let args = cli::Cli::parse();
    let state = Arc::new(State::new(args).await);
    let ok = warp::path("ok").map(|| warp::reply);

    // 404 page in case page is not found
    let page_404 = filters::any::any().map(|| {
        println!("ok!");
        Response::builder()
            .status(404)
            .body("<!DOCTYPE html><head><head/><body>404</body>")
    });

    // websocket connection for when user is in app.
    let api_v0_ws = warp::path("ws")
        .and(filters::ws::ws())
        .and(state::add_default(state.clone()))
        .and_then(websocket::handler)
        .recover(|_| "");

    let api_v0_poll_messages = warp::path("poll_messages")
        .and(state::add_default(state.clone()))
        .map(todo!());

    // version 0 of the api
    let api_v0 = warp::path("api/v0/").and(api_v0_ws.or(api_v0_poll_messages));

    // create adrress from command line arguments
    let addr = std::net::SocketAddr::new(
        // use localhost as
        args.host
            .unwrap_or(std::net::Ipv4Addr::new(127, 0, 0, 1).into()),
        args.port.unwrap_or(String::from("80")).parse().unwrap(),
    );
    let req = ok.or(api_v0).or(page_404).map(|_| panic!("unreachable!"));

    warp::serve(req).run(addr).await;
}
