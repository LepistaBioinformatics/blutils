mod cmds;
mod models;

use anyhow::Result;
use clap::Subcommand;
use cmds::{blast, check, db_builder};
use models::{cli_launcher::CliLauncher, log_format::LogFormat};
use std::{path::PathBuf, str::FromStr};
use tracing::debug;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Subcommand, Debug)]
#[command(author, version, about, long_about = None)]
enum Opts {
    /// Build the blast database as a pre-requisite for the blastn command.
    BuildDb(db_builder::Arguments),

    /// Execute the parallel blast and run consensus algorithm
    Blastn(blast::Arguments),

    /// Check `Blutils` dependencies
    Check(check::Arguments),
}

#[tracing::instrument(name = "expose_runtime_arguments")]
pub fn expose_runtime_arguments() {
    let args: Vec<_> = std::env::args().collect();
    debug!("{:?}", args.join(" "));
}

fn main() -> Result<()> {
    let args = CliLauncher::<Opts>::parse();

    // ? -----------------------------------------------------------------------
    // ? Configure logger
    // ? -----------------------------------------------------------------------

    let log_level = args.log_level.unwrap_or("error".to_string());

    let (non_blocking, _guard) = match args.log_file {
        //
        // If no log file is provided, log to stderr
        //
        None => tracing_appender::non_blocking(std::io::stderr()),
        //
        // If a log file is provided, log to the file
        //
        Some(file) => {
            let mut log_file = PathBuf::from(file);

            let binding = log_file.to_owned();
            let parent_dir = binding
                .parent()
                .expect("Log file parent directory not found");

            match args.log_format {
                LogFormat::Jsonl => {
                    log_file.set_extension("jsonl");
                }
                LogFormat::Ansi => {
                    log_file.set_extension("log");
                }
            };

            if log_file.exists() {
                std::fs::remove_file(&log_file)?;
            }

            let file_name =
                log_file.file_name().expect("Log file name not found");

            let file_appender =
                tracing_appender::rolling::never(parent_dir, file_name);

            tracing_appender::non_blocking(file_appender)
        }
    };

    let tracing_config = tracing_subscriber::fmt()
        .event_format(
            fmt::format()
                .with_level(true)
                .with_target(false)
                .with_thread_ids(true)
                .with_file(false)
                .with_line_number(false),
        )
        .with_writer(non_blocking)
        .with_env_filter(EnvFilter::from_str(log_level.as_str()).unwrap());

    match args.log_format {
        LogFormat::Ansi => tracing_config.pretty().init(),
        LogFormat::Jsonl => tracing_config.json().init(),
    };

    // ? -----------------------------------------------------------------------
    // ? Get command line arguments
    // ? -----------------------------------------------------------------------

    expose_runtime_arguments();

    // ? -----------------------------------------------------------------------
    // ? Fire up the command
    // ? -----------------------------------------------------------------------

    match args.opts {
        Opts::BuildDb(sub_args) => match sub_args.build {
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
        Opts::Blastn(blast_args) => {
            match blast_args.run_blast {
                blast::Commands::RunWithConsensus(sub_args) => {
                    blast::run_blast_and_build_consensus_cmd(
                        sub_args,
                        args.threads,
                    )
                }
                blast::Commands::BuildConsensus(args) => {
                    blast::build_consensus_cmd(args)
                }
                blast::Commands::BuildTabular(args) => {
                    blast::build_tabular_cmd(args)
                }
            };
        }
        Opts::Check(check_args) => {
            match check_args.check_host {
                check::Commands::Linux => check::check_host_requirements_cmd(),
            };
        }
    };

    Ok(())
}
