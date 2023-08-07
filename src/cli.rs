use clap::Parser;

#[derive(Parser)]
pub struct Cli {
    #[arg(long, default_value_t = 80)]
    pub http_port: u16,

    #[arg(long, default_value_t = 443)]
    pub https_port: u16,

    #[arg(long, default_value_t = [0, 0, 0, 0].into())]
    pub server_host: std::net::IpAddr,

    #[arg(long)]
    pub verbose: bool,

    #[arg(long)]
    pub files: Option<String>,

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

    #[arg(long, default_value_t = String::from("/etc/letsencrypt/live/esfokk.nl/fullchain.pem"))]
    pub ssl_certificate: String,
    #[arg(long, default_value_t = String::from("/etc/letsencrypt/live/esfokk.nl/privkey.pem"))]
    pub ssl_key: String,

    #[arg(long, default_value_t = String::from("/php"))]
    pub php_root: String,

    #[arg(long)]
    /// enable dev mode
    pub dev: bool,

    #[arg(long)]
    pub tokio_console: bool,
}
