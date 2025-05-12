use axum::{routing::get, Router};
use clap::Parser;

use notify::{Config, Watcher};
#[allow(unused)]
mod api;
mod cli;
mod db;
mod message;
mod prelude;
mod state;
mod webpages;
//mod websocket;
use prelude::*;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    let args = {
        let mut a = if let Ok(f) = std::fs::read_to_string(
            #[cfg(debug_assertions)]
            "dev.config.toml",
            #[cfg(not(debug_assertions))]
            "config.toml",
        ) {
            toml::from_str(&f).unwrap()
        } else {
            cli::Cli::default()
        };
        a.update_from(std::env::args_os());
        a
    };
    if args.tokio_console {
        console_subscriber::init();
    } else {
        let filter = tracing_subscriber::filter::Targets::new()
            .with_target("bane_server", args.log_level)
            .with_default(tracing::Level::INFO);
        let tracing_fmt = tracing_subscriber::fmt::layer();
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_fmt)
            .init();
    }
    info!("args: {args:#?}");
    let state = Arc::new(State::new(args).await);
    *state.watcher.write().await = Some(signal_handler(state.clone()));
    // let api_v0_router = Router::new()
    //     .route("/poll_messages", get())
    //     .with_state(state);
    /*
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
    ));*/
    // let base =
    //     .and(state::add_default(state.clone()))
    //     .and_then(|path: warp::path::FullPath, state: Arc<State>| {
    //         webpages::handler(path, state).then(|e| async {
    //             e.map(|e| Response::new(e))
    //                 .map_err(|_| warp::reject::not_found())
    //         })
    //     });

    let router = Router::new()
        .route(
            "/",
            get(|query, state| {
                webpages::webpages_handler(axum::extract::Path(String::new()), query, state)
            }),
        )
        .route("/script/websocket/*path", get(webpages::websocket_scripts))
        .route("/script/*path", get(webpages::scripts))
        .route("/*path", get(webpages::webpages_handler))
        .route("/api/filestream", get(api::filestream::filestream_handler))
        .route(
            "/api/filestream/:uuid",
            get(api::filestream::filestream_reciever_handler),
        )
        // .route("/api/v0", Route)
        .with_state(state.clone());

    let addr = std::net::SocketAddr::new(
        // use localhost as
        std::net::Ipv4Addr::new(127, 0, 0, 1).into(),
        state.args.http_port,
    );
    if state.args.dev {
        info!("running dev mode!");
    } else {
        info!("running normal mode!")
    }
    let tcp_listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|_| panic!("failed to bind to {}", &addr));

    axum::serve(tcp_listener, router.into_make_service())
        .await
        .unwrap();
    //warp::serve(req).run(addr).await;
}

#[tracing::instrument(skip(state))]
fn signal_handler(state: Arc<State>) -> notify::INotifyWatcher {
    let mut watcher = {
        let state = state.clone();
        notify::INotifyWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                if res.is_ok() {
                    let mut lock = state.tera.blocking_write();
                    match lock.as_mut().map(|e| e.full_reload()) {
                        Some(Err(e)) => error!("terra error: {}", e),
                        Some(Ok(_)) => info!(
                            "terra reload: reason: {:#?}\n{:#?}",
                            res.unwrap(),
                            lock.as_mut()
                                .unwrap()
                                .get_template_names()
                                .collect::<Vec<_>>()
                        ),
                        _ => {
                            *lock = {
                                match webpages::tera(&state.args) {
                                    Err(e) => {
                                        error!("Tera error: {e}");
                                        None
                                    }
                                    e => e.ok(),
                                }
                            }
                        }
                    };
                }
            },
            Config::default().with_follow_symlinks(true),
        )
        .unwrap()
    };
    let _ = watcher.watch(
        std::path::Path::new(&state.args.template_dir),
        notify::RecursiveMode::Recursive,
    );
    watcher
}
