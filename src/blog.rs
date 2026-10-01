use axum::{extract::Path, response::Response};
use serde::Serialize;
use tracing::error;

use crate::webpages::get_path_under_dir;

const BLOG_DIR: &str = "./blogs/";

pub async fn blog(
    Path(mut p): Path<String>,
    // State(state): State<Arc<crate::State>>,
) -> Result<Response, http::StatusCode> {
    if !p.ends_with(".md") {
        p.push_str(".md");
    }
    let p = get_path_under_dir(std::path::Path::new(BLOG_DIR), &p)
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

#[derive(Serialize)]
pub struct Blog {
    title: String,
    href: String,
}

pub fn blogs() -> Vec<Blog> {
    std::fs::read_dir(BLOG_DIR)
        .map(|e| {
            e.filter_map(|e| {
                let blog = e.ok()?;
                let stdout = std::process::Command::new("pandoc")
                    .arg("-ttitle.lua")
                    .arg(blog.path())
                    .output()
                    .ok()?
                    .stdout;
                let title = String::from_utf8_lossy(&stdout);
                Some(Blog {
                    title: title.to_string(),
                    href: "blog/".to_string() + &blog.file_name().to_string_lossy(),
                })
            })
            .collect()
        })
        .unwrap_or_default()
}
