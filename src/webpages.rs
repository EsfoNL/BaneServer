use crate::prelude::*;
use crate::state::State;
use async_trait::async_trait;
use axum::{
    body::Bytes,
    extract::{FromRequest, Path, Query},
    response::IntoResponse,
};
use reqwest::Request;
use std::{collections::HashMap, ops::DerefMut, sync::Arc};
use tera::Tera;
use tower::Service;

pub fn tera(path: &str) -> Option<tera::Tera> {
    match Tera::new(&format!("{}/**", path)) {
        Ok(mut tera) => {
            tera.register_function("command", crate::webpages::command);
            tera.register_function("sh", crate::webpages::shell_command);
            info!(
                "loaded terra templates: {:#?}",
                tera.get_template_names().collect::<Vec<&str>>()
            );
            Some(tera)
        }
        Err(err) => {
            error!("terra error: {err}");
            None
        }
    }
}

#[instrument(skip(state, request))]
pub async fn webpages_handler(
    Path(path): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    axum::extract::State(state): axum::extract::State<Arc<State>>,
    RequestExtractor(request): RequestExtractor,
) -> axum::response::Response {
    if let Some(lock) = state.tera.read().await.as_ref() {
        let mut cont = state.context.clone();
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
                return axum::response::Response::new(axum::body::boxed(axum::body::Body::from(
                    e.into_bytes(),
                )));
            }
            Err(tera::Error {
                kind: tera::ErrorKind::TemplateNotFound(_),
                ..
            }) => {
                // continue to file service
            }
            Err(_) => return http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
    if let Some(ref mut e) = state.dir_service.lock().await.deref_mut() {
        let req = e.call(request).await.unwrap();
        match req.status() {
            http::StatusCode::OK => {
                debug!("file matched: {}", &path);
            }
            _ => {
                info!("no route matches!");
            }
        };
        return req.into_response();
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

#[instrument(skip(state))]
pub async fn gitea_handler(
    axum::extract::State(state): axum::extract::State<Arc<State>>,
    RequestExtractor(request): RequestExtractor,
) -> Result<
    http::Response<
        axum::body::StreamBody<impl futures::Stream<Item = Result<Bytes, reqwest::Error>>>,
    >,
    (),
> {
    // http::Request != reqwest::Request
    let mut url = reqwest::Url::parse("http://127.0.0.1:3000").unwrap();
    let path = request.uri().path()["/gitea".len()..].to_owned();
    let query = request.uri().query();

    // .ok_or(())?.as_str()["/gitea".len()..].to_owned();
    debug!("{} -> {path}", request.uri());
    if let Some(ref v) = query {
        debug!("query: {v}");
    }
    url.set_path(&path);
    url.set_query(query);
    let mut new_request = Request::new(request.method().clone(), url);
    *new_request.headers_mut() = request.headers().to_owned();
    *new_request.body_mut() = Some(request.into_body().into());
    let e = state
        .reqwest_client
        .execute(new_request)
        .await
        .map_err(|e| {
            warn!("{}", e);
        })?;
    debug!("response: {e:#?}");
    let headers = e.headers().to_owned();
    let status = e.status().to_owned();
    let mut actual_res =
        axum::response::Response::new(axum::body::StreamBody::new(e.bytes_stream()));
    *actual_res.headers_mut() = headers;
    *actual_res.status_mut() = status;
    Ok(actual_res)
}

pub struct RequestExtractor(axum::http::Request<axum::body::Body>);

#[async_trait]
impl FromRequest<Arc<State>, axum::body::Body> for RequestExtractor {
    type Rejection = ();

    async fn from_request(
        req: axum::http::Request<axum::body::Body>,
        _state: &Arc<State>,
    ) -> Result<Self, ()> {
        return Ok(RequestExtractor(req));
    }
}
