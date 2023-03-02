use crate::state::State;
use std::sync::Arc;
use tera::{Context, Tera};

pub async fn root(state: Arc<State>) -> impl warp::Reply {
    let error = String::from("error");
    let mut context = Context::new();
    context.insert("users", &1);
    let generated_html = state.tera.read().await.as_ref().map_or(error.clone(), |e| {
        e.render("root.html", &context).unwrap_or(error.clone())
    });
    warp::http::response::Builder::new().body(generated_html)
}
