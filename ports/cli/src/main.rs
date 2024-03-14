mod cmds;

use clap::Parser;
use cmds::{blast, check, db_builder};
use std::str::FromStr;
use tracing::debug;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
enum Cli {
    /// Build the blast database as a pre-requisite for the blastn command.
    BuildDb(db_builder::Arguments),

    /// Execute the parallel blast and run consensus algorithm
    Blastn(blast::Arguments),

    /// Check `Blutils` dependencies
    Check(check::Arguments),
}

/// Get the command line arguments.
fn get_arguments() {
    let args: Vec<_> = std::env::args().collect();
    debug!("{:?}", args.join(" "));
}

fn main() {
    let log_level = std::env::var("RUST_LOG").unwrap_or("error".to_string());

    let tracing_config = tracing_subscriber::fmt()
        .event_format(
            fmt::format()
                // don't include levels in formatted output
                .with_level(true)
                // don't include targets
                .with_target(false)
                .compact(),
        )
        .with_env_filter(EnvFilter::from_str(log_level.as_str()).unwrap());

    if std::env::var("RUST_LOG_FORMAT").unwrap_or("".to_string()) == "json" {
        tracing_config.json().init();
    } else {
        tracing_config.with_ansi(true).init();
    }

    get_arguments();

    match Cli::parse() {
        Cli::BuildDb(sub_args) => match sub_args.build {
            db_builder::Commands::Blu(args) => {
                db_builder::run_blast_and_build_consensus_cmd(args)
            }
            db_builder::Commands::Qiime2(args) => {
                db_builder::build_qiime_db_from_blutils_db_cmd(args)
            }
            db_builder::Commands::Kraken2(args) => {
                db_builder::build_kraken_db_from_ncbi_files_cmd(args)
            }
        },
        Cli::Blastn(blast_args) => {
            match blast_args.run_blast {
                blast::Commands::RunWithConsensus(args) => {
                    blast::run_blast_and_build_consensus_cmd(args)
                }
                blast::Commands::BuildConsensus(args) => {
                    blast::build_consensus_cmd(args)
                }
                blast::Commands::BuildTabular(args) => {
                    blast::build_tabular_cmd(args)
                }
            };
        }
        Cli::Check(check_args) => {
            match check_args.check_host {
                check::Commands::Linux => check::check_host_requirements_cmd(),
            };
        }
    };
}
