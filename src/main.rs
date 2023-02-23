use clap::Parser;

use warp::http::Response;
use warp::{filters, Filter};

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
    let ok = warp::path::end().then(ok);

    // websocket connection for when user is in app.
    let api_v0_ws = warp::path("ws")
        .and(filters::ws::ws())
        .and(state::add_default(state.clone()))
        .and(state::add_token_id())
        .then(websocket::handler)
        .boxed();

    let api_v0_poll_messages = warp::path("poll_messages")
        .and(state::add_default(state.clone()))
        .and(state::add_token_id())
        .map(|_state, _token, _id| warp::reply())
        .boxed();

    let api_v0_login = warp::path("login")
        .and(state::add_default(state.clone()))
        .and(warp::header("Email"))
        .and(warp::header("Password"))
        .then(api::login);

    let api_v0_register = warp::path("register")
        .and(state::add_default(state.clone()))
        .and(warp::header("Email"))
        .and(warp::header("Password"))
        .and(warp::header("Name"))
        .then(api::register);

    // version 0 of the api
    let api_v0 = warp::path("api")
        .and(warp::path("v0"))
        .and(
            api_v0_poll_messages
                .or(api_v0_ws)
                .or(api_v0_login)
                .or(api_v0_register)
                .or(ok),
        )
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
        let http_server = warp::serve(redirect).bind(addr2);
        tokio::spawn(https_server);
        http_server.await;
    }
}

async fn ok() -> impl warp::Reply {
    use maud::*;
    warp::http::Response::builder().body(
        html! {
            h1 {
                "hello world!"
            }
        }
        .into_string(),
    )
}
