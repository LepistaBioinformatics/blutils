use super::log_format::LogFormat;

use clap::{FromArgMatches, Parser, Subcommand};

#[derive(Parser, Debug)]
#[clap(name = "blu", version, author, about)]
pub struct CliLauncher<T: FromArgMatches + Subcommand> {
    #[clap(subcommand)]
    pub opts: T,

    #[clap(long)]
    pub log_level: Option<String>,

    #[clap(long)]
    pub log_file: Option<String>,

    #[clap(long, default_value = "ansi")]
    pub log_format: LogFormat,

    #[clap(short, long, default_value = "1")]
    pub threads: Option<usize>,
}

impl<T: FromArgMatches + Subcommand> CliLauncher<T> {
    pub fn parse() -> Self {
        Self::parse_from(std::env::args())
    }
}
