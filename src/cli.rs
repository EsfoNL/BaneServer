use clap::Parser;

#[derive(Parser)]
pub struct Cli {
    #[arg(short, long)]
    pub port: Option<String>,

    #[arg(short, long)]
    pub verbose: bool,

    #[arg(short, long)]
    pub files: Option<String>,

    #[arg(short, long)]
    pub sqlserver: Option<String>,

    #[arg(short, long)]
    pub sqlport: Option<u16>,

    #[arg(short, long)]
    pub sqlpassword: Option<String>,

    #[arg(short, long)]
    pub sqlusername: Option<String>,
}
