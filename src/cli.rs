use clap::Parser;

#[derive(Debug, Parser)]
pub struct App {
    #[arg(short, long)]
    pub config: Option<String>,
}
