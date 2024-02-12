use clap::Parser;

#[derive(Debug, Parser)]
pub struct App {
    #[arg(
        short,
        long,
        value_name = "PATH",
        default_value_t = String::from("/etc/fand/config.toml")
    )]
    pub config: String,
}
