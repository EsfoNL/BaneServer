use clap::Parser;
use futures::FutureExt;
use notify::Watcher;
use std::str::FromStr;
use warp::http::Response;
use warp::{filters, Filter};
use webpages::gitea_handler;

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

    if state.args.tokio_console {
        console_subscriber::init();
    }
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

    let static_path = warp::fs::dir(state.args.static_dir.clone());
    let gitea = warp::path("gitea")
        .and(warp::path::tail())
        .and(warp::method())
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes())
        .and(state::add_default(state.clone()))
        .and_then(gitea_handler);
    // create adrress from command line arguments
    let req = gitea.or(warp::get().and(
        base.or(api_v0)
            .or(static_path.clone())
            .or(warp::any().map(|| Response::builder().status(404).body(String::from("404")))),
    ));

    if state.args.dev {
        println!("running dev mode!");
        let addr = std::net::SocketAddr::new(
            // use localhost as
            std::net::Ipv4Addr::new(127, 0, 0, 1).into(),
            state.args.http_port,
        );
        warp::serve(req).run(addr).await;
    } else {
        let http = std::net::SocketAddr::new(
            // use localhost as
            state.args.server_host.clone(),
            state.args.http_port.clone(),
        );
        let https = std::net::SocketAddr::new(
            // use localhost as
            state.args.server_host.clone(),
            state.args.https_port.clone(),
        );

        let https_server = warp::serve(req)
            .tls()
            .key_path(state.args.ssl_key.clone())
            .cert_path(state.args.ssl_certificate.clone())
            .bind(https);
        let redirect = warp::filters::path::full().map(|path: warp::path::FullPath| {
            warp::redirect(
                warp::http::Uri::from_str(&("https://esfokk.nl".to_owned() + path.as_str()))
                    .unwrap(),
            )
        });
        let http_server = warp::serve(static_path.or(redirect)).bind(http);
        tokio::spawn(https_server);
        http_server.await;
    }
}

fn signal_handler(state: Arc<State>) -> impl Watcher {
    let mut watcher = {
        let state = state.clone();
        notify::recommended_watcher(move |res| {
            if let Ok(_) = res {
                eprintln!("{:?}", state.tera.blocking_write().full_reload());
            }
        })
        .unwrap()
    };
    let _ = watcher.watch(
        std::path::Path::new(&state.args.template_dir),
        notify::RecursiveMode::Recursive,
    );
    watcher
}
