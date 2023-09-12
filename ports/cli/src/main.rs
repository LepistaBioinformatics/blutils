mod cmds;

use clap::Parser;
use cmds::{blast, check, db_builder};
use log::info;
use std::io::Write;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
enum Cli {
    BuildDb(db_builder::BuildDatabaseArguments),

    /// Execute the parallel blast and run consensus algorithm.
    Blast(blast::Arguments),

    /// Check `Blutils` dependencies.
    Check(check::Arguments),
}

/// Get the command line arguments.
fn get_arguments() {
    let args: Vec<_> = std::env::args().collect();
    info!("{:?}", args.join(" "));
}

fn main() {
    env_logger::builder()
        .format(|buf, record| {
            writeln!(buf, "[ {} ]  {}", record.level(), record.args())
        })
        .init();

    get_arguments();

    let args = Cli::parse();

    match args {
        Cli::BuildDb(sub_args) => {
            db_builder::run_blast_and_build_consensus_cmd(sub_args)
        }

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
