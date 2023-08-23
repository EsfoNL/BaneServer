use std::{
    io::{Read, Write},
    net::SocketAddr,
    pin,
    task::Poll,
    time::Duration,
};

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc {};

use axum::{
    routing::{get, MethodFilter, MethodRouter},
    Router,
};
use clap::Parser;
use hyper::server::accept::Accept;
use tokio::io::{AsyncRead, AsyncWrite};

use notify::Watcher;
use rustls::{ServerConfig, ServerConnection};
use tracing::{debug, error, info};
use webpages::gitea_handler;
//mod api;
mod cli;
mod db;
mod message;
mod prelude;
mod state;
mod webpages;
//mod websocket;
use prelude::*;

#[tokio::main]
async fn main() {
    let args = cli::Cli::parse();
    let state = Arc::new(State::new(args).await);
    *state.watcher.write().await = Some(signal_handler(state.clone()));

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

    let router: Router<(), axum::body::Body> = Router::new()
        .route(
            "/gitea",
            MethodRouter::new().on(MethodFilter::all(), gitea_handler),
        )
        .route(
            "/gitea/*key",
            MethodRouter::new().on(MethodFilter::all(), gitea_handler),
        )
        .route(
            "/",
            get(|state, req| {
                webpages::common_handler(axum::extract::Path(String::new()), state, req)
            }),
        )
        .route("/*path", get(webpages::common_handler))
        .with_state(state.clone());

    /*    let listener_state = state.clone();
        tokio::spawn(async move {
            let sock = tokio::net::TcpListener::bind(std::net::SocketAddr::new(
                std::net::Ipv4Addr::new(127, 0, 0, 1).into(),
                5000,
            ))
            .await
            .unwrap();
            loop {
                let con_state = listener_state.clone();
                if let Ok((mut con, _)) = sock.accept().await {
                    tokio::spawn(async move {
                        let mut buf = vec![];
                        loop {
                            let val = con.read_u8().await.unwrap();
                            if val != 0 {
                                buf.push(val)
                            } else {
                                let string = String::from_utf8_lossy(&buf).into_owned();
                                buf.clear();
                                if let Ok(v) = tracing::Level::from_str(&string) {
                                    let _ = con_state.filter_handle.lock().await.modify(|e| {
                                        *e = tracing::level_filters::LevelFilter::from_level(v)
                                    });
                                }
                            }
                        }
                    });
                }
            }
        });
    */
    if state.args.dev {
        info!("running dev mode!");
        let addr = std::net::SocketAddr::new(
            // use localhost as
            std::net::Ipv4Addr::new(127, 0, 0, 1).into(),
            state.args.http_port,
        );
        axum::Server::bind(&addr)
            .serve(router.into_make_service())
            .await
            .unwrap();
        //warp::serve(req).run(addr).await;
    } else {
        let _http = std::net::SocketAddr::new(
            // use localhost as
            state.args.server_host.clone(),
            state.args.http_port.clone(),
        );
        let https = std::net::SocketAddr::new(
            // use localhost as
            state.args.server_host.clone(),
            state.args.https_port.clone(),
        );
        let tls_server_config = Arc::new(
            rustls::ServerConfig::builder()
                .with_safe_defaults()
                .with_no_client_auth()
                .with_single_cert(
                    load_certificates_from_pem(&state.args.ssl_certificate).unwrap(),
                    load_private_key_from_file(&state.args.ssl_key).unwrap(),
                )
                .unwrap(),
        );

        let tls_acceptor = TlsAcceptor::new(https, &tls_server_config).await;
        hyper::server::Server::builder(tls_acceptor)
            .serve(router.into_make_service())
            .await
            .unwrap();
        // let acceptor =
        //         .map(|e| {});
        // let stream = tok

        /*
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
        http_server.await;*/
    }
}

struct TlsAcceptor {
    tcp_listener: tokio::net::TcpListener,
    config: Arc<ServerConfig>,
}

impl TlsAcceptor {
    async fn new(socket_addr: SocketAddr, config: &Arc<rustls::ServerConfig>) -> Self {
        Self {
            tcp_listener: tokio::net::TcpListener::bind(socket_addr).await.unwrap(),
            config: config.clone(),
        }
    }
}

impl Accept for TlsAcceptor {
    type Conn = TlsStream;

    type Error = std::io::Error;

    fn poll_accept(
        self: pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        match self.tcp_listener.poll_accept(cx) {
            Poll::Ready(Ok(e)) => Poll::Ready(Some(Ok(TlsStream::new(&self.config, e.0)))),
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}

struct TlsStream {
    close: tokio::sync::mpsc::Sender<()>,
    rustls_con: Arc<tokio::sync::Mutex<ServerConnection>>,
    read_waker_sender: tokio::sync::mpsc::Sender<std::task::Waker>,
    write_notify: Arc<tokio::sync::Notify>,
    task: Option<tokio::task::JoinHandle<anyhow::Result<()>>>,
}

impl Drop for TlsStream {
    fn drop(&mut self) {
        let _ = self.close.try_send(());
        let task = self.task.take();
        if let Some(e) = task {
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(10)).await;
                e.abort();
            });
        }
    }
}

impl TlsStream {
    fn new(config: &Arc<rustls::ServerConfig>, con: tokio::net::TcpStream) -> Self {
        let (read_sender, read_reciever) = tokio::sync::mpsc::channel(16);
        let notify = Arc::new(tokio::sync::Notify::new());
        let n_2 = notify.clone();
        let (close, close_recv) = tokio::sync::mpsc::channel(1);
        let rustls_con = Arc::new(tokio::sync::Mutex::new(
            ServerConnection::new(config.clone()).unwrap(),
        ));
        let sec_con = rustls_con.clone();
        let task = tokio::spawn(tls_task(con, rustls_con, read_reciever, close_recv, notify));
        Self {
            close,
            rustls_con: sec_con,
            read_waker_sender: read_sender,
            write_notify: n_2,
            task: Some(task),
        }
    }
}

#[tracing::instrument(skip_all)]
async fn tls_task(
    mut con: tokio::net::TcpStream,
    rustls_con: std::sync::Arc<tokio::sync::Mutex<rustls::ServerConnection>>,
    mut read_reciever: tokio::sync::mpsc::Receiver<std::task::Waker>,
    mut close_reciever: tokio::sync::mpsc::Receiver<()>,
    notify: std::sync::Arc<tokio::sync::Notify>,
) -> Result<(), anyhow::Error> {
    let mut read_wakers: Vec<std::task::Waker> = vec![];
    let mut write_wakers: Vec<std::task::Waker> = vec![];
    let mut buf = vec![];
    loop {
        tokio::select! {
            w = read_reciever.recv() => {
                debug!("waker recv!");
                if let Some(w) = w {
                    read_wakers.push(w);
                } else {
                    return Ok(())
                }
            },
            e = con.readable() => {
                e?;
                if let Ok(_) = con.try_read_buf(&mut buf) {
                    rustls_con.lock().await.read_tls(&mut &buf[..])?;
                    buf.clear();
                    rustls_con.lock().await.process_new_packets()?;
                    for i in read_wakers.iter() {
                        i.wake_by_ref();
                    }
                    read_wakers.clear();
                    rustls_con.lock().await.write_tls(&mut buf)?;
                    debug!("data written");
                    tokio::io::AsyncWriteExt::write(&mut con, &buf).await?;
                    buf.clear();
                }
            },
            _ = notify.notified() => {
                debug!("data written");
                if rustls_con.lock().await.write_tls(&mut buf)? > 0 {
                    loop {
                        let read_len = tokio::io::AsyncWriteExt::write(&mut con, &buf).await?;
                        if read_len < buf.len() {
                            buf = Vec::from(&buf[read_len..]);
                        } else {
                            break;
                        }
                    }
                    for i in write_wakers.iter() {
                        i.wake_by_ref();
                    }
                }
            },

            _ = close_reciever.recv() => {
                debug!("closing!");

                tokio::io::AsyncWriteExt::shutdown(&mut con).await?;
                return Ok(());
            }
        }
    }
}

impl AsyncRead for TlsStream {
    fn poll_read(
        self: pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let lock = self.rustls_con.try_lock();
        if let Ok(mut lock) = lock {
            if !lock.wants_read() {
                return Poll::Ready(lock.reader().read(buf.initialize_unfilled()).map(|n| {
                    buf.advance(n);
                    ()
                }));
            }
        }
        match self.read_waker_sender.try_send(cx.waker().clone()) {
            Ok(_) => Poll::Pending,
            Err(_) => Poll::Ready(Err(std::io::ErrorKind::ConnectionAborted.into())),
        }
    }
}

impl AsyncWrite for TlsStream {
    fn poll_write(
        self: pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let lock = self.rustls_con.try_lock();
        if let Ok(mut lock) = lock {
            let res = lock.writer().write(buf);
            self.write_notify.notify_one();
            return Poll::Ready(res);
        } else {
            let lock_2 = self.rustls_con.clone();
            let waker = cx.waker().clone();
            tokio::spawn(async move {
                let _ = lock_2.lock().await;
                waker.wake();
            });
            Poll::Pending
        }
    }

    fn poll_flush(
        self: pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        let _ = self.close.try_send(());
        Poll::Ready(Ok(()))
    }
}

#[tracing::instrument(skip(state))]
fn signal_handler(state: Arc<State>) -> notify::INotifyWatcher {
    let mut watcher = {
        let state = state.clone();
        notify::recommended_watcher(move |res| {
            if let Ok(_) = res {
                let mut lock = state.tera.blocking_write();
                match lock.as_mut().map(|e| e.full_reload()) {
                    Some(Err(e)) => error!("terra error: {}", e),
                    Some(Ok(_)) => info!(
                        "terra reload: {:#?}",
                        lock.as_mut()
                            .unwrap()
                            .get_template_names()
                            .collect::<Vec<_>>()
                    ),
                    _ => *lock = State::tera(&state.args.template_dir),
                };
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

/// taken from [rustls docs](https://docs.rs/rustls/latest/rustls/struct.PrivateKey.html#Examples)
fn load_private_key_from_file(
    path: &str,
) -> Result<rustls::PrivateKey, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(&path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut keys = rustls_pemfile::pkcs8_private_keys(&mut reader)?;

    match keys.len() {
        0 => Err(format!("No PKCS8-encoded private key found in {path}").into()),
        1 => Ok(rustls::PrivateKey(keys.remove(0))),
        _ => Err(format!("More than one PKCS8-encoded private key found in {path}").into()),
    }
}
/// taken from [rustls docs](https://docs.rs/rustls/latest/rustls/struct.Certificate.html#Examples)
fn load_certificates_from_pem(path: &str) -> std::io::Result<Vec<rustls::Certificate>> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let certs = rustls_pemfile::certs(&mut reader)?;

    Ok(certs.into_iter().map(rustls::Certificate).collect())
}
