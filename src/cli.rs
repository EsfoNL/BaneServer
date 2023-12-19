use clap::Parser;

#[derive(Parser, Debug)]
pub struct Cli {
    #[arg(long, default_value_t = 80)]
    pub http_port: u16,

    #[arg(long, default_value_t = [0, 0, 0, 0].into())]
    pub server_host: std::net::IpAddr,

    #[arg(long)]
    pub verbose: bool,

    #[arg(long)]
    pub files: Option<std::path::PathBuf>,

    #[arg(long, default_value_t = String::from("127.0.0.1"))]
    pub sqlhost: String,

    #[arg(long, default_value_t = 3306)]
    pub sqlport: u16,

    #[arg(long)]
    pub sqlpassword: Option<String>,

    #[arg(long)]
    pub sqlusername: Option<String>,

    #[arg(long, default_value_t = String::from("/www"))]
    pub static_dir: String,

    #[arg(long, default_value_t = String::from("templates"))]
    pub template_dir: String,

    #[arg(long)]
    /// enable dev mode
    pub dev: bool,

    #[arg(long)]
    pub tokio_console: bool,

    #[arg(long, default_value_t = 3000)]
    pub gitea_port: u16,

    #[arg(long, default_value_t = tracing::Level::WARN)]
    pub log_level: tracing::Level,
}
