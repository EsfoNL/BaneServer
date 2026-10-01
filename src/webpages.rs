use crate::prelude::*;
use axum::{
    body::Body,
    extract::{Path, Query},
    response::{IntoResponse, Response},
};
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use regex::Regex;
use std::{
    cell::RefCell, collections::HashMap, os::unix::fs::MetadataExt, path::PathBuf, process::Stdio,
    str::FromStr,
};
use tera::{Kwargs, Tera};
use tokio::io::AsyncReadExt;
use tracing::info_span;

#[derive(Default)]
struct TeraContext {
    response_headers: HeaderMap,
}

thread_local! {
    static TERA_CTX: RefCell<TeraContext> = RefCell::new(TeraContext::default());
}

pub fn tera(cli: &Cli) -> Result<tera::Tera, tera::Error> {
    let mut tera = Tera::new();
    tera.register_function("command", command);
    tera.register_function("sh", shell_command);
    tera.register_function("files", files(cli));
    tera.register_function("obj", obj);
    tera.register_function("cors", cors);
    tera.register_function("headers", headers);
    tera.register_filter("ansi_to_html", ansi_to_html);
    tera.register_test("pub_root", is_pub_root(cli));
    tera.register_filter("date", tera_contrib::dates::date);
    tera.register_filter(
        "filesizeformat",
        tera_contrib::filesize_format::filesize_format,
    );
    tera.global_context().insert("blogs", &crate::blog::blogs());

    match tera.load_from_glob(&format!("{}/**", cli.template_dir)) {
        Ok(()) => {
            info!(
                "loaded terra templates: {:#?}",
                tera.get_template_names().collect::<Vec<&str>>()
            );
            Ok(tera)
        }
        Err(e) => Err(e),
    }
}

fn ansi_to_html(value: &str, _: Kwargs, _: &tera::State) -> tera::TeraResult<tera::Value> {
    ansi_to_html::convert(value)
        .map_err(|_| tera::Error::message(String::from("failed to convert from ansi to html")))
        .map(|e| tera::Value::safe_string(&e))
}

fn headers(args: Kwargs, _: &tera::State) -> tera::TeraResult<tera::Value> {
    TERA_CTX.with_borrow_mut(|ctx| {
        debug!("here!");
        for (key, val) in args
            .iter()
            .map(|(a, b)| (a, b.as_str().ok_or(tera::Error::message("wrong value"))))
        {
            let val = val?;
            let key: String = key
                .as_str()
                .ok_or(tera::Error::message("invalid key"))?
                .chars()
                .map(|e| match e {
                    '_' => '-',
                    e => e,
                })
                .collect();
            debug!("inserting {key}: {val} into headers");
            ctx.response_headers.insert(
                HeaderName::from_str(key.as_str())
                    .map_err(|e| tera::Error::message(e.to_string()))?,
                HeaderValue::from_str(val).map_err(|e| tera::Error::message(e.to_string()))?,
            );
        }
        Ok(tera::Value::none())
    })
}

fn cors(args: Kwargs, _: &tera::State) -> tera::TeraResult<tera::Value> {
    TERA_CTX.with(|e| {
        if let Some(val) = args.get::<Vec<&str>>("orgs")? {
            let response_headers = &mut e.borrow_mut().response_headers;
            response_headers.insert(
                "Access-Control-Allow-Origin",
                HeaderValue::from_str(&val.into_iter().fold(
                    String::new(),
                    |mut acc: String, v: &str| {
                        acc.push_str(v);
                        acc.push_str(", ");
                        acc
                    },
                ))
                .unwrap(),
            );
        }
        Ok(tera::Value::none())
    })
}

fn obj(args: Kwargs, _: &tera::State) -> tera::TeraResult<HashMap<String, tera::Value>> {
    Ok(args
        .iter()
        .filter_map(|e| Some((e.0.as_str()?.to_owned(), e.1.clone())))
        .collect())
}

pub fn tera_context(cli: &Cli) -> tera::Context {
    let mut context = tera::Context::new();
    context.insert("pub_file_prefix", &cli.pub_file_prefix);
    context
}

fn explore_dir(state: &State) -> Result<PathBuf, http::StatusCode> {
    let Some(base_path) = state.args.pub_dir.clone() else {
        return Err(http::StatusCode::INTERNAL_SERVER_ERROR);
    };
    let mut actual_base_path = std::env::current_dir().unwrap();
    actual_base_path.push(&base_path);
    let Ok(canon_base_path) = base_path.canonicalize() else {
        error!("invalid pub dir: {base_path:#?}");
        return Err(http::StatusCode::INTERNAL_SERVER_ERROR);
    };

    Ok(canon_base_path)
}

#[instrument(skip(state))]
pub async fn webpages_handler(
    Path(path): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    axum::extract::State(state): axum::extract::State<Arc<State>>,
) -> Result<axum::response::Response, http::StatusCode> {
    let mut lock = state.tera.write().await;
    let Some(tera) = lock.as_mut() else {
        info!("tera not loaded");
        return Err(http::StatusCode::NOT_FOUND);
    };
    let mut cont = state.context.clone();
    if let Some(path) = query.get("path") {
        let canon_base_path = explore_dir(&state)?;
        let mut full_path = canon_base_path.clone();
        full_path.push(path);
        let Ok(cannoned_path) = full_path.canonicalize() else {
            error!("invalid path: {path:#?}");
            return Err(http::StatusCode::INTERNAL_SERVER_ERROR);
        };

        if cannoned_path != full_path {
            let Ok(server_path) = cannoned_path.strip_prefix(&canon_base_path) else {
                return Err(http::StatusCode::NOT_FOUND);
            };
            return Ok(http::Response::builder()
                .status(http::StatusCode::FOUND)
                .header(
                    "Location",
                    format!("?path={}", server_path.to_string_lossy()),
                )
                .body(axum::body::Body::empty())
                .unwrap());
        };
    }
    cont.insert("query", &query);
    match tera.render(
        if path.is_empty() {
            "root.html"
        } else {
            path.as_str()
        },
        &cont,
    ) {
        Ok(e) => {
            debug!("tera matched: {}", &path);
            let mut res = axum::response::Response::new(axum::body::Body::from(e.into_bytes()));
            TERA_CTX.with(|e| {
                res.headers_mut()
                    .extend(e.borrow_mut().response_headers.drain())
            });
            return Ok(res);
        }
        Err(e) => {
            error!("terra error: {e:?}");
            return Err(http::StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
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

pub fn command(kwargs: Kwargs, _: &tera::State) -> Result<tera::Value, tera::Error> {
    let mut command = std::process::Command::new(
        kwargs
            .get::<&str>("name")?
            .ok_or(tera::Error::message("program name not provided"))?,
    );
    if let Ok(Some(args)) = kwargs.get::<Vec<String>>("args")
    // .map(|e| e.iter().filter_map(|e| e.as_str()))
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

pub fn shell_command(args: Kwargs, _: &tera::State) -> Result<tera::Value, tera::Error> {
    let mut command = std::process::Command::new("sh");
    command.arg("-c");
    command.arg(args.get::<&str>("command")?.unwrap());
    Ok(to_json_or_string(
        std::str::from_utf8(command.output().unwrap().stdout.as_slice()).unwrap(),
    ))
}

fn to_json_or_string(string: &str) -> tera::Value {
    tera::Value::from_serializable(
        &serde_json::from_str::<serde_json::Value>(string)
            .unwrap_or(serde_json::Value::String(string.to_owned())),
    )
}

type TeraBoxedTester =
    Box<dyn Send + Sync + Fn(&tera::Value, Kwargs, &tera::State) -> Result<bool, tera::Error>>;

fn is_pub_root(cli: &Cli) -> TeraBoxedTester {
    let mut path = std::env::current_dir().unwrap();
    let failed_to_construct_path: TeraBoxedTester = Box::new(|_, _, _| {
        error!("Tera pub dir not set");
        Err(tera::Error::message("tera pub dir not set"))
    });
    let Some(add_path) = cli.pub_dir.as_ref().map(std::path::PathBuf::from) else {
        return failed_to_construct_path;
    };
    path.push(add_path);
    let Ok(path) = path.canonicalize() else {
        return failed_to_construct_path;
    };
    info!("pub dir absolute path: {path:?}");
    Box::new(move |value: &tera::Value, _, _| {
        debug!("value: {value:?}");
        Ok(value
            .as_str()
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
    Box<dyn Sync + Send + Fn(Kwargs, &tera::State) -> Result<tera::Value, tera::Error>>;

#[derive(serde::Serialize)]
struct FileInfo {
    filename: String,
    path: String,
    is_file: bool,
    size: u64,
    atime: i64,
}

fn files(cli: &Cli) -> TeraBoxedFn {
    let mut path = std::env::current_dir().unwrap();
    let Some(add_path) = cli.pub_dir.as_ref().map(std::path::PathBuf::from) else {
        return Box::new(|_, _| {
            error!("Tera pub dir not set");
            Err(tera::Error::message("tera pub dir not set"))
        });
    };
    path.push(add_path);
    // info!("pub dir absolute path: {path:?}");

    Box::new(move |args: Kwargs, _| {
        info_span!("files").in_scope(|| {
            let mut new_path = path.clone();
            let s = args.get("path")?.unwrap_or("");
            new_path.push(s);
            debug!("s: {s}");

            new_path = new_path
                .canonicalize()
                .map_err(|_| tera::Error::message(format!("not a valid path: {new_path:#?}")))?;
            if !new_path.starts_with(&path) {
                return Err(tera::Error::message(format!(
                    "not a valid path: {new_path:#?}"
                )));
            };
            let mut res = std::fs::read_dir(&new_path)
                .map_err(|_| tera::Error::message(format!("not a valid path: {new_path:#?}")))?
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
                .map(|(res_path, v)| FileInfo {
                    filename: v.file_name().into_string().unwrap(),
                    is_file: v.file_type().unwrap().is_file(),
                    size: v.metadata().unwrap().size(),
                    atime: v.metadata().unwrap().atime(),
                    path: res_path.to_string_lossy().into_owned(),
                })
                .collect::<Vec<_>>();
            if let Some(option) = args.get("sort")? {
                match option {
                    "atime" => res.sort_unstable_by_key(|e| e.atime),
                    "size" => res.sort_unstable_by_key(|e| e.size),
                    _ => (),
                }
            }
            if let Some(true) = args.get("rev")? {
                // info!("reversed!");
                res.reverse();
            }

            if let Some(ext) = args.get("filter")? {
                let re = Regex::new(ext).unwrap();
                res.retain(|e| re.is_match(&e.filename));
            }
            tera::Value::try_from_serializable(&res)
        })
    })
}

/// tries to get a path strictly under the [base_path], else returns None
#[tracing::instrument]
pub fn get_path_under_dir(base_path: &std::path::Path, path: &str) -> Option<PathBuf> {
    let base_path = base_path.canonicalize().unwrap();
    let mut full_path = base_path.to_owned();
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
    let Ok((out, mut errout)) = tokio::process::Command::new(&path)
        .env("QUERY", query_json)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|e| {
            e.stdout
                .zip(e.stderr)
                .ok_or(std::io::ErrorKind::NotFound.into())
        })
    else {
        return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    // res.headers_mut().insert(
    //     "Content-Type",
    //     HeaderValue::from_static("text/plain; charset=UTF-8"),
    // );
    tokio::spawn(async move {
        let mut buf = [0; 256];
        while let Ok(v @ 1..) = errout.read(&mut buf).await {
            info!("{path:?} stderr: {}", String::from_utf8_lossy(&buf[0..v]));
        }
    });
    axum::response::Response::new(axum::body::Body::from_stream(
        tokio_util::io::ReaderStream::new(out),
    ))

    // String::from_utf8_lossy().into_owned().into_response()
}

pub async fn file_scripts(
    Path(path): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    axum::extract::State(state): axum::extract::State<Arc<State>>,
) -> Result<axum::response::Response, http::StatusCode> {
    let Some(path) = get_path_under_dir(&state.args.scripts_path, &path) else {
        return Err(http::StatusCode::NOT_FOUND);
    };

    let Ok(query_json) = serde_json::to_string(&query) else {
        return Err(http::StatusCode::INTERNAL_SERVER_ERROR);
    };

    let Ok(mut child) = tokio::process::Command::new(path)
        .env("QUERY", query_json)
        .stdout(std::process::Stdio::piped())
        .spawn()
    else {
        return Err(http::StatusCode::INTERNAL_SERVER_ERROR);
    };

    axum::response::Response::builder()
        .header(http::header::CONTENT_TYPE, "application/octet-stream")
        .body(Body::from_stream(tokio_util::io::ReaderStream::new(
            child.stdout.take().unwrap(),
        )))
        .map_err(|_| http::StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn download_zip(
    Path(path): Path<String>,
    axum::extract::State(state): axum::extract::State<Arc<State>>,
) -> Result<axum::response::Response, http::StatusCode> {
    // println!("hit");
    let pub_dir_path = explore_dir(&state)?;
    let path_at_dir =
        get_path_under_dir(&pub_dir_path, &path).ok_or(http::StatusCode::NOT_FOUND)?;

    let mut proc = tokio::process::Command::new("zip")
        // since the pub diris a dir it is guaranteed to have a parent path
        .args(["-r", "-"])
        .arg(path_at_dir.file_name().unwrap())
        .current_dir(path_at_dir.parent().unwrap())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let base_name = path_at_dir
        .file_name()
        .ok_or(http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Response::builder()
        .header(http::header::CONTENT_TYPE, "application/octet-stream")
        .header(
            http::header::CONTENT_DISPOSITION,
            format!(
                "attachment; filename=\"{}.zip\"",
                base_name.to_string_lossy()
            ),
        )
        .body(Body::from_stream(tokio_util::io::ReaderStream::new(
            proc.stdout
                .take()
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
        )))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn asset(Path(path): Path<String>) -> Result<Response, Response> {
    let r404 = || Response::builder().status(404).body(().into()).unwrap();
    debug!("path: {path}");
    let path = get_path_under_dir(&PathBuf::from("assets"), &path).ok_or_else(r404)?;
    let mime_type = if let Some(ext) = path.extension() {
        match &*ext.to_string_lossy() {
            "js" => "text/javascript",
            _ => "",
        }
    } else {
        ""
    };
    let mut response = Response::new(Body::from_stream(tokio_util::io::ReaderStream::new(
        tokio::fs::File::open(&path).await.map_err(|_| r404())?,
    )));
    response.headers_mut().insert(
        http::header::CONTENT_TYPE,
        HeaderValue::from_static(mime_type),
    );
    Ok(response)
}
