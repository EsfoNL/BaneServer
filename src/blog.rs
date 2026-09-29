use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use tracing::error;

use crate::webpages::get_path_under_dir;

pub async fn blog(
    Path(p): Path<String>,
    // State(state): State<Arc<crate::State>>,
) -> Result<Response, http::StatusCode> {
    let p = get_path_under_dir(std::path::Path::new("./blogs/"), &p)
        .ok_or(http::StatusCode::NOT_FOUND)?;
    let out = tokio::process::Command::new("pandoc")
        .args([
            "--data-dir=./pandoc-templates",
            "-fmarkdown",
            "-thtml5",
            "-s",
        ])
        .arg(&p)
        .output()
        .await
        .map_err(|_| http::StatusCode::INTERNAL_SERVER_ERROR)?;
    // if !out.status.success() {
    error!("{}", String::from_utf8_lossy(&out.stderr));
    // }
    Response::builder()
        .header(http::header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(out.stdout.into())
        .map_err(|_| http::StatusCode::INTERNAL_SERVER_ERROR)
}
