use std::ops::DerefMut;

use clap::Parser;

use futures::{FutureExt, TryFutureExt};
use notify::Watcher;
use warp::http::Response;
use warp::{filters, Filter, Reply};

mod api;
mod cli;
mod db;
mod message;
mod prelude;
mod state;
mod webpages;
mod websocket;
use prelude::*;

#[tokio::main]
async fn main() {
    let args = cli::Cli::parse();
    let state = Arc::new(State::new(args).await);
    let base = warp::path::full()
        .and(state::add_default(state.clone()))
        .and_then(|path: warp::path::FullPath, state: Arc<State>| {
            webpages::handler(path, state).then(|e| async {
                e.map(|e| Response::new(e))
                    .map_err(|_| warp::reject::not_found())
            })
        });

    *state.watcher.lock().unwrap() = Some(Box::new(signal_handler(state.clone())));

    // websocket connection for when user is in app.
    let api_v0_ws = warp::path("ws")
        .and(filters::ws::ws())
        .and(state::add_default(state.clone()))
        .and(api::add_token_id())
        .then(websocket::handler)
        .boxed();

    let api_v0_poll_messages = warp::path("poll_messages")
        .and(state::add_default(state.clone()))
        .and(api::add_token_id())
        .then(api::poll_messages)
        .boxed();

    let api_v0_login = warp::path("login")
        .and(state::add_default(state.clone()))
        .and(warp::header("email"))
        .and(warp::header("password"))
        .then(api::login);

    let api_v0_register = warp::path("register")
        .and(state::add_default(state.clone()))
        .and(warp::header("email"))
        .and(warp::header("password"))
        .and(warp::header("name"))
        .then(api::register);

    let api_v0_query_name = warp::path("query_name")
        .and(state::add_default(state.clone()))
        .and(warp::header("name"))
        .then(api::query_name);

    let api_v0_query_id = warp::path("query_id")
        .and(state::add_default(state.clone()))
        .and(warp::header("id"))
        .then(api::query_id);

    let api_v0_refresh_token = warp::path("refresh")
        .and(state::add_default(state.clone()))
        .and(warp::header("id"))
        .and(warp::header("refresh_token"))
        .then(api::refresh_token);

    // version 0 of the api
    let api_v0 = warp::path("api")
        .and(warp::path("v0"))
        .and(
            api_v0_poll_messages
                .or(api_v0_ws)
                .or(api_v0_login)
                .or(api_v0_register)
                .or(api_v0_query_name)
                .or(api_v0_query_id)
                .or(api_v0_refresh_token),
        )
        .boxed();

    let static_path = warp::fs::dir(
        state
            .args
            .static_dir
            .clone()
            .unwrap_or(String::from("/www")),
    );

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
        base.or(api_v0)
            .or(static_path.clone())
            .or(warp::any().map(|| Response::builder().status(404).body(String::from("404")))),
    );

    if cfg!(debug_assertions) {
        warp::serve(req).run(addr).await;
    } else {
        let addr1 = std::net::SocketAddr::new(
            // use localhost as
            [185, 107, 90, 38].into(),
            443,
        );
        let addr2 = std::net::SocketAddr::new(
            // use localhost as
            [185, 107, 90, 38].into(),
            80,
        );

        let https_server = warp::serve(req)
            .tls()
            .key_path("/etc/letsencrypt/live/esfokk.nl/privkey.pem")
            .cert_path("/etc/letsencrypt/live/esfokk.nl/fullchain.pem")
            .bind(addr1);
        let redirect = warp::filters::path::full().map(|path: warp::path::FullPath| {
            warp::redirect(
                format!("https://esfokk.nl{}", path.as_str())
                    .parse::<warp::http::Uri>()
                    .unwrap_or(warp::http::Uri::from_static("https://esfokk.nl")),
            )
        });
        let http_server = warp::serve(static_path.or(redirect)).bind(addr2);
        tokio::spawn(https_server);
        http_server.await;
    }
}

fn signal_handler(state: Arc<State>) -> impl Watcher {
    let mut watcher = notify::recommended_watcher(move |res| {
        if let Ok(_) = res {
            eprintln!("{:?}", state.tera.blocking_write().full_reload());
        }
    })
    .unwrap();
    watcher.watch(
        std::path::Path::new("templates"),
        notify::RecursiveMode::Recursive,
    );
    watcher
}
