use axum::{routing::get, Router};
use clap::Parser;

use notify::{Config, Event, EventKind, Watcher};
mod admin;
mod api;
mod cli;
mod db;
mod message;
mod prelude;
mod script;
mod state;
mod webpages;
// mod websocket;
use prelude::*;
use tower::{Layer, Service, ServiceBuilder};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{admin::admin_router, webpages::download_zip};

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

    let router = Router::new()
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .route(
            "/",
            get(|query, state| {
                webpages::webpages_handler(axum::extract::Path(String::new()), query, state)
            }),
        )
        .nest("/admin", admin_router().with_state(state.clone()))
        .route("/script/websocket/{*path}", {
            let state = state.clone();
            get(move |p, q, s, ws| async {
                let state = state;
                script::websocket_scripts(
                    std::path::Path::new(&state.args.scripts_path),
                    p,
                    q,
                    s,
                    ws,
                )
                .await
            })
        })
        .route("/script/file/{*path}", get(webpages::file_scripts))
        .route("/script/{*path}", get(webpages::scripts))
        .route("/download-zip/{*path}", get(download_zip))
        .route("/asset/{*path}", get(webpages::asset))
        .route("/{*path}", get(webpages::webpages_handler))
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
                    let ev = res.unwrap();
                    if let Event {
                        kind: EventKind::Access(_) | EventKind::Any | EventKind::Other,
                        ..
                    } = ev
                    {
                        return;
                    }
                    let mut lock = state.tera.blocking_write();
                    match lock.as_mut().map(|e| e.full_reload()) {
                        Some(Err(e)) => error!("terra error: {}", e),
                        Some(Ok(_)) => info!(
                            "terra reload: reason: {:#?}\n{:#?}",
                            ev,
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
