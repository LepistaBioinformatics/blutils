extern crate blul;
mod cmds;

use clap::Parser;
use cmds::{blast, check};
use env_logger::Builder;
use log::LevelFilter;
use std::io::Write;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
enum Cli {
    Blast(blast::Arguments),
    Check(check::Arguments),
}

fn main() {
    Builder::new()
        .format(|buf, record| {
            writeln!(buf, "[ {} ]  {}", record.level(), record.args())
        })
        .filter(None, LevelFilter::Info)
        .init();

    let args = Cli::parse();

    match args {
        Cli::Blast(sub_args) => {
            match sub_args.run_blast {
                blast::Commands::RunWithConsensus(args) => {
                    blast::run_blast_and_build_consensus_cmd(args)
                }
            };
        }
        Cli::Check(sub_args) => {
            match sub_args.check_host {
                check::Commands::Linux => check::check_host_requirements_cmd(),
            };
        }
    };
}
