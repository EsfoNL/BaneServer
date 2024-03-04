use crate::prelude::*;
use async_trait::async_trait;
use axum::{
    body::Bytes,
    extract::{FromRequest, Path, Query},
    response::IntoResponse,
};
use reqwest::Request;
use std::collections::HashMap;
use tera::Tera;

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

#[instrument(skip(state))]
pub async fn webpages_handler(
    Path(path): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    axum::extract::State(state): axum::extract::State<Arc<State>>,
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
