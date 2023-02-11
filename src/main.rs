use clap::Parser;

use warp::http::Response;
use warp::{filters, Filter, Reply};

mod api;
mod cli;
mod db;
mod message;
mod prelude;
mod state;
mod websocket;
use prelude::*;

#[tokio::main]
async fn main() {
    let args = cli::Cli::parse();
    let state = Arc::new(State::new(args).await);
    let ok = warp::path("ok").map(|| warp::reply());

    // websocket connection for when user is in app.
    let api_v0_ws = warp::path("ws")
        .and(filters::ws::ws())
        .and(state::add_default(state.clone()))
        .and(state::add_token())
        .then(websocket::handler)
        .boxed();

    let api_v0_poll_messages = warp::path("poll_messages")
        .and(state::add_default(state.clone()))
        .and(state::add_token())
        .map(|_state, _token| warp::reply())
        .boxed();

    let api_v0_login = warp::path("login")
        .and(state::add_default(state.clone()))
        .and(warp::header("Email"))
        .and(warp::header("Password"))
        .then(api::login);

    // version 0 of the api
    let api_v0 = warp::path("api")
        .and(warp::path("v0"))
        .and(api_v0_poll_messages.or(api_v0_ws).or(api_v0_login).or(ok))
        .boxed();

    // create adrress from command line arguments
    let addr = std::net::SocketAddr::new(
        // use localhost as
        state
            .args
            .server_host
            .clone()
            .unwrap_or(std::net::Ipv4Addr::new(127, 0, 0, 1).into()),
        state
            .args
            .port
            .clone()
            .unwrap_or(String::from("80"))
            .parse()
            .unwrap(),
    );
    let req = warp::get().and(
        ok.or(api_v0)
            .or(warp::any().map(|| Response::builder().status(404).body(String::from("404")))),
    );
    warp::serve(req).run(addr).await;
}
