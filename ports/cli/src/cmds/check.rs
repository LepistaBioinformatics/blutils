use blul_core::use_cases::check_host_requirements;
use clap::Parser;

#[derive(Parser, Debug)]
pub(crate) struct Arguments {
    #[clap(subcommand)]
    pub check_host: Commands,
}

#[derive(Parser, Debug)]
pub(crate) enum Commands {
    Linux,
}

pub(crate) fn check_host_requirements_cmd() {
    if let Err(err) = check_host_requirements(None) {
        panic!("{err}");
    }
}
