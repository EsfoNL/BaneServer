use crate::state::State;
use std::{collections::HashMap, sync::Arc};
use tera::{Context, Tera};
use warp::{path::FullPath, Rejection};

pub async fn handler(path: FullPath, state: Arc<State>) -> Result<String, &'static str> {
    let lock = state.tera.read().await;
    let path = path.as_str();
    if state.args.verbose {
        eprintln!("{path}");
    }
    if path == "/" || path == "" {
        return lock
            .render("root.html", &state.context)
            .ok()
            .ok_or("root render failed");
    }
    lock.render(&path[1..], &state.context).map_err(|e| {
        if state.args.verbose {
            eprintln!("terra error: {e:?}");
        }
        "tera error"
    })
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
