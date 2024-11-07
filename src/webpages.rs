use crate::prelude::*;
use axum::{
    body::Bytes,
    extract::{ws::Message, FromRequest, Path, Query},
    response::{IntoResponse, Response},
};
use futures::{SinkExt, StreamExt, TryStreamExt};
use http::{response::Parts, HeaderValue, StatusCode};
use reqwest::Request;
use std::{
    borrow::{Borrow, BorrowMut},
    collections::HashMap,
    ops::{Deref, DerefMut},
    os::unix::process::CommandExt,
    path::PathBuf,
    process::Command,
    time::Duration,
};
use tera::Tera;
use tokio::io::AsyncReadExt;
use tracing::info_span;

pub fn tera(cli: &Cli) -> Result<tera::Tera, tera::Error> {
    match Tera::new(&format!("{}/**", cli.template_dir)) {
        Ok(mut tera) => {
            tera.register_function("command", command);
            tera.register_function("sh", shell_command);
            tera.register_function("files", files(cli));
            tera.register_tester("pub_root", is_pub_root(cli));
            info!(
                "loaded terra templates: {:#?}",
                tera.get_template_names().collect::<Vec<&str>>()
            );
            Ok(tera)
        }
        Err(e) => Err(e),
    }
}

pub fn tera_context(cli: &Cli) -> tera::Context {
    let mut context = tera::Context::new();
    context.insert("pub_file_prefix", &cli.pub_file_prefix);
    context
}

#[instrument(skip(state))]
pub async fn webpages_handler(
    Path(path): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    axum::extract::State(state): axum::extract::State<Arc<State>>,
) -> axum::response::Response {
    if let Some(lock) = state.tera.read().await.as_ref() {
        let mut cont = state.context.clone();
        if let Some(path) = query.get("path") {
            let Some(base_path) = state.args.pub_dir.clone() else {
                return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
            };
            let mut actual_base_path = std::env::current_dir().unwrap();
            actual_base_path.push(&base_path);
            let Ok(mut canon_base_path) = base_path.canonicalize() else {
                error!("invalid pub dir: {base_path:#?}");
                return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
            };
            let mut full_path = canon_base_path.clone();
            full_path.push(path);
            let Ok(cannoned_path) = full_path.canonicalize() else {
                error!("invalid path: {path:#?}");
                return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
            };

            if cannoned_path != full_path {
                let Ok(server_path) = cannoned_path.strip_prefix(&canon_base_path) else {
                    return http::StatusCode::NOT_FOUND.into_response();
                };
                return http::Response::builder()
                    .status(http::StatusCode::FOUND)
                    .header(
                        "Location",
                        format!("?path={}", server_path.to_string_lossy().to_string()),
                    )
                    .body(axum::body::Body::empty())
                    .unwrap();
            };
        }
        cont.insert("query", &query);
        match lock.render(
            if path.is_empty() {
                "root.html"
            } else {
                path.as_str()
            },
            &cont,
        ) {
            Ok(e) => {
                debug!("tera matched: {}", &path);
                return axum::response::Response::new(axum::body::Body::from(e.into_bytes()));
            }
            Err(e) => {
                error!("terra error: {e:?}");
                return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    }

    info!("404");
    return http::StatusCode::NOT_FOUND.into_response();
}

/*
pub async fn handler(
    Path(path): Path<String>,
    axum::extract::State(state): axum::extract::State<Arc<State>>,
) -> Result<Vec<u8>, &'static str> {
    if let Some(ref lock) = *state.tera.read().await {
        Ok(lock
            .render(&path, &state.context)
            .map_err(|e| {
                warn!("tera error: {}", e);
                "tera error"
            })
            .map(|e| e.into_bytes())?)
    } else {
        Err("tera not loaded")
    }
}*/

pub fn command(args: &HashMap<String, tera::Value>) -> Result<tera::Value, tera::Error> {
    let mut command = std::process::Command::new(
        args.get("name")
            .ok_or(tera::Error::msg("program name not provided"))?
            .as_str()
            .ok_or(tera::Error::msg("program name not a string"))?,
    );
    if let Some(args) = args
        .get("args")
        .and_then(|e| e.as_array())
        .map(|e| e.iter().filter_map(|e| e.as_str()))
    {
        for i in args {
            command.arg(i);
        }
    }
    let handle = command.output()?;
    Ok(to_json_or_string(
        std::str::from_utf8(handle.stdout.as_slice()).unwrap(),
    ))
}

pub fn shell_command(args: &HashMap<String, tera::Value>) -> Result<tera::Value, tera::Error> {
    let mut command = std::process::Command::new("sh");
    command.arg("-c");
    command.arg(args.get("command").unwrap().as_str().unwrap());
    Ok(to_json_or_string(
        std::str::from_utf8(command.output().unwrap().stdout.as_slice()).unwrap(),
    ))
}

fn to_json_or_string(string: &str) -> serde_json::Value {
    serde_json::from_str(string).unwrap_or(serde_json::json!(string))
}

type TeraBoxedTester =
    Box<dyn Send + Sync + Fn(Option<&tera::Value>, &[tera::Value]) -> Result<bool, tera::Error>>;

fn is_pub_root(cli: &Cli) -> TeraBoxedTester {
    let mut path = std::env::current_dir().unwrap();
    let failed_to_construct_path: TeraBoxedTester = Box::new(|_, _| {
        error!("Tera pub dir not set");
        Err(tera::Error::msg("tera pub dir not set"))
    });
    let Some(add_path) = cli.pub_dir.as_ref().map(std::path::PathBuf::from) else {
        return failed_to_construct_path;
    };
    path.push(add_path);
    let Ok(path) = path.canonicalize() else {
        return failed_to_construct_path;
    };
    info!("pub dir absolute path: {path:?}");
    Box::new(move |value: Option<&tera::Value>, _: &[tera::Value]| {
        debug!("value: {value:?}");
        Ok(value
            .and_then(|e| e.as_str())
            .and_then(|e| {
                let mut cur = path.clone();
                cur.push(e);
                let val = cur.canonicalize().ok();
                debug!("val: {val:?}");
                val
            })
            .map(|e| e == path)
            .unwrap_or(true))
    })
}

type TeraBoxedFn =
    Box<dyn Sync + Send + Fn(&HashMap<String, tera::Value>) -> Result<tera::Value, tera::Error>>;

fn files(cli: &Cli) -> TeraBoxedFn {
    info!("help!");
    let mut path = std::env::current_dir().unwrap();
    let Some(add_path) = cli.pub_dir.as_ref().map(std::path::PathBuf::from) else {
        return Box::new(|_| {
            error!("Tera pub dir not set");
            Err(tera::Error::msg("tera pub dir not set"))
        });
    };
    path.push(add_path);
    info!("pub dir absolute path: {path:?}");

    Box::new(move |args: &HashMap<String, tera::Value>| {
        info_span!("files").in_scope(|| {
            let mut new_path = path.clone();
            let s = args.get("path").and_then(|e| e.as_str()).unwrap_or("");
            new_path.push(s);
            debug!("s: {s}");

            new_path = new_path
                .canonicalize()
                .map_err(|e| tera::Error::msg(format!("not a valid path: {new_path:#?}")))?;
            if !new_path.starts_with(&path) {
                return Err(tera::Error::msg(format!("not a valid path: {new_path:#?}")));
            };
            let res = std::fs::read_dir(&new_path)
                .map_err(|_| tera::Error::msg(format!("not a valid path: {new_path:#?}")))?
                .filter_map(Result::ok)
                .map(|e| {
                    (
                        std::path::PathBuf::from(
                            e.path()
                                .canonicalize()
                                .unwrap()
                                .strip_prefix(&path)
                                .unwrap(),
                        ),
                        e,
                    )
                })
                .map(|(res_path, v)| {
                    let mut map = tera::Map::new();
                    map.insert(
                        String::from("filename"),
                        res_path
                            .file_name()
                            .map(|e| tera::Value::String(e.to_string_lossy().to_string()))
                            .unwrap_or(tera::Value::Null),
                    );
                    map.insert(
                        String::from("path"),
                        tera::Value::String(res_path.to_string_lossy().to_string()),
                    );

                    map.insert(
                        String::from("isFile"),
                        tera::Value::Bool(v.file_type().unwrap().is_file()),
                    );
                    // map.insert(
                    //     String::from("filename"),
                    //     res_path
                    //         .file_name()
                    //         .map(|e| tera::Value::String(e.to_string_lossy().to_string()))
                    //         .unwrap_or(tera::Value::Null),
                    // );
                    tera::Value::Object(map)
                })
                .collect::<Vec<_>>();
            Ok(tera::Value::Array(res))
        })
    })
}

/// tries to get a path stricktly under the [base_path], else returns None
fn get_path_under_dir(base_path: &PathBuf, path: &str) -> Option<PathBuf> {
    let mut full_path = base_path.clone();
    full_path.push(path);
    let Ok(full_path) = full_path.canonicalize() else {
        return None;
    };
    if !full_path.starts_with(base_path) || full_path.is_relative() {
        return None;
    }

    Some(full_path)
}

pub async fn scripts(
    Path(path): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    axum::extract::State(state): axum::extract::State<Arc<State>>,
) -> axum::response::Response {
    let Some(path) = get_path_under_dir(&state.args.scripts_path, &path) else {
        return http::StatusCode::NOT_FOUND.into_response();
    };
    let Ok(query_json) = serde_json::to_string(&query) else {
        return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let Ok(out) = tokio::process::Command::new(path)
        .env("QUERY", query_json)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|e| e.stdout.ok_or(std::io::ErrorKind::NotFound.into()))
    else {
        return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    // res.headers_mut().insert(
    //     "Content-Type",
    //     HeaderValue::from_static("text/plain; charset=UTF-8"),
    // );
    axum::response::Response::new(axum::body::Body::from_stream(
        tokio_util::io::ReaderStream::new(out),
    ))

    // String::from_utf8_lossy().into_owned().into_response()
}

pub async fn websocket_scripts(
    Path(path): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    axum::extract::State(state): axum::extract::State<Arc<State>>,
    ws: axum::extract::WebSocketUpgrade,
) -> axum::response::Response {
    info!("ws called: {path}");
    let Some(path) = get_path_under_dir(&state.args.scripts_path, &path) else {
        return http::StatusCode::NOT_FOUND.into_response();
    };
    let Ok(query_json) = serde_json::to_string(&query) else {
        return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let Ok(mut child) = tokio::process::Command::new(path)
        .env("QUERY", query_json)
        .stdout(std::process::Stdio::piped())
        .spawn()
    else {
        return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    if child.stdout.is_none() {
        return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    ws.on_upgrade(|mut ws| async move {
        let mut out = child.stdout.take().unwrap();
        let mut buff = [0u8; 256];

        loop {
            tokio::select! {
                _ = ws.recv() => {
                    break;
                },
                data = out.read(&mut buff) => {
                    let Ok(l) = data else {
                        break;
                    };
                    if ws.send(Message::Text(
                        String::from_utf8_lossy(&buff[..l]).to_string()
                    )).await.is_err() {
                        break;
                    };
                }

            }
        }

        info!("aborted!");

        let _ = ws.close().await;
        info!("websockets closed");
        let _ = child
            .kill()
            .await
            .inspect_err(|e| info!("websocket error: {e:?}"));
        info!("child ended")
    })
}
