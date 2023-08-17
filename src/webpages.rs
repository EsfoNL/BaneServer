use crate::state::{self, State};
use async_trait::async_trait;
use axum::{
    body::Bytes,
    extract::{FromRequest, Path},
    response::Html,
};
use futures::StreamExt;
use http::Uri;
use reqwest::{header::HeaderMap, Method, Request, RequestBuilder, Response, Version};
use std::{collections::HashMap, sync::Arc};
use tera::{Context, Tera};
use tracing::{debug, info, instrument, warn};

#[instrument(skip(state))]
pub async fn handler(
    Path(path): Path<String>,
    axum::extract::State(state): axum::extract::State<Arc<State>>,
) -> Result<Html<String>, &'static str> {
    if let Some(ref lock) = *state.tera.read().await {
        Ok(Html(lock.render(&path[..], &state.context).map_err(
            |e| {
                warn!("tera error: {}", e);
                "tera error"
            },
        )?))
    } else {
        Err("tera not loaded")
    }
}
pub async fn root_handler(
    axum::extract::State(state): axum::extract::State<Arc<State>>,
) -> Result<Html<String>, &'static str> {
    Ok(Html(if let Some(ref tera) = *state.tera.read().await {
        tera.render("root.html", &state.context)
            .ok()
            .ok_or("root render failed")?
    } else {
        return Err("terra not loaded");
    }))
}

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
        std::str::from_utf8(&command.output().unwrap().stdout.as_slice()).unwrap(),
    ))
}

fn to_json_or_string(string: &str) -> serde_json::Value {
    let value = serde_json::from_str(string).unwrap_or(serde_json::json!(string));
    value
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
    let path = request.uri().path_and_query().ok_or(())?.as_str()["/gitea".len()..].to_owned();
    debug!("{} -> {path}", request.uri());
    url.set_path(&path);
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
