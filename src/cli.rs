use clap::Parser;

#[derive(Parser)]
pub struct Cli {
    #[arg(long)]
    pub port: Option<String>,

    #[arg(long)]
    pub server_host: Option<std::net::IpAddr>,

    #[arg(long)]
    pub verbose: bool,

    #[arg(long)]
    pub files: Option<String>,

    #[arg(long)]
    pub sqlserver: Option<String>,

    #[arg(long)]
    pub sqlport: Option<u16>,

    #[arg(long)]
    pub sqlpassword: Option<String>,

    #[arg(long)]
    pub sqlusername: Option<String>,

    #[arg(long)]
    pub static_dir: Option<String>,
}
