use std::str::FromStr;

use clap::Parser;
use serde::{Deserialize, Deserializer};

const LEVEL_STRINGS: [&str; 5] = ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"];

struct V;
impl serde::de::Visitor<'_> for V {
    type Value = tracing::Level;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an value in the range ERROR..=TRACE")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        tracing::Level::from_str(v).map_err(|_| E::unknown_variant(v, &LEVEL_STRINGS))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        tracing::Level::from_str(&v).map_err(|_| E::unknown_variant(&v, &LEVEL_STRINGS))
    }
}

fn deserialize_tracing_level<'de, D>(d: D) -> Result<tracing::Level, D::Error>
where
    D: Deserializer<'de>,
{
    d.deserialize_str(V)
}

impl Default for Cli {
    fn default() -> Self {
        Self {
            http_port: 80,
            server_host: [127, 0, 0, 1].into(),
            verbose: false,
            files: None,
            sqlhost: String::from("127.0.0.1"),
            sqlport: 3306,
            sqlpassword: None,
            sqlusername: None,
            static_dir: String::from("/www"),
            template_dir: String::from("templates"),
            dev: false,
            tokio_console: false,
            gitea_port: 3000,
            log_level: tracing::Level::INFO,
        }
    }
}

#[derive(Parser, Debug, Deserialize)]
#[serde(default)]
pub struct Cli {
    #[arg(long)]
    pub http_port: u16,

    #[arg(long)]
    pub server_host: std::net::IpAddr,

    #[arg(long)]
    pub verbose: bool,

    #[arg(long)]
    pub files: Option<std::path::PathBuf>,

    #[arg(long)]
    pub sqlhost: String,

    #[arg(long)]
    pub sqlport: u16,

    #[arg(long)]
    pub sqlpassword: Option<String>,

    #[arg(long)]
    pub sqlusername: Option<String>,

    #[arg(long)]
    pub static_dir: String,

    #[arg(long)]
    pub template_dir: String,

    #[arg(long)]
    /// enable dev mode
    pub dev: bool,

    #[arg(long)]
    pub tokio_console: bool,

    #[arg(long)]
    pub gitea_port: u16,

    #[arg(long)]
    #[serde(deserialize_with = "deserialize_tracing_level")]
    pub log_level: tracing::Level,
}
