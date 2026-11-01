mod cmds;
mod models;

use anyhow::{anyhow, Context, Result};
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
            let parent_dir = binding.parent().with_context(|| {
                format!(
                    "Invalid log file path (no parent directory): {binding:?}"
                )
            })?;

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

            let file_name = log_file.file_name().with_context(|| {
                format!("Invalid log file path (no file name): {log_file:?}")
            })?;

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
        .with_env_filter(EnvFilter::from_str(log_level.as_str()).map_err(
            |err| anyhow!("Invalid --log-level {log_level:?}: {err}"),
        )?);

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

    // `--threads` caps the whole process, not only the BLAST step (BUG-R40).
    blul_core::use_cases::limit_threads(args.threads.unwrap_or(1))
        .map_err(|err| anyhow!("{err}"))?;

    // BUG-R42: commands return errors instead of panicking. Returning from
    // `main` prints `Error: <message>`, exits with code 1 and drops the log
    // guard, so buffered log lines are flushed.
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
        Opts::Blastn(blast_args) => match blast_args.run_blast {
            blast::Commands::RunWithConsensus(sub_args) => {
                blast::run_blast_and_build_consensus_cmd(sub_args, args.threads)
            }
            blast::Commands::BuildConsensus(args) => {
                blast::build_consensus_cmd(args)
            }
            blast::Commands::BuildTabular(args) => {
                blast::build_tabular_cmd(args)
            }
        },
        Opts::Check(check_args) => match check_args.check_host {
            check::Commands::Linux => check::check_host_requirements_cmd(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blul_core::domain::dtos::file_or_stdin::FileOrStdin;
    use clap::{error::ErrorKind, CommandFactory, Parser};

    // Every parse below passes an explicit `query`/`blu_result` path: the
    // positional default "-" goes through `FileOrStdin::from_str`, which flips
    // the process-global `STDIN_HAS_BEEN_USED` and would make later parses in
    // this binary fail depending on test order. Clap evaluates defaults before
    // validating required args, so this also applies to error cases.

    type Cli = CliLauncher<Opts>;

    const RUN: [&str; 3] = ["blu", "blastn", "run-with-consensus"];

    fn run_with_consensus(extra: &[&str]) -> Result<Cli, clap::Error> {
        let mut argv: Vec<&str> = RUN.to_vec();
        argv.extend_from_slice(&["query.fna"]);
        argv.extend_from_slice(extra);
        Cli::try_parse_from(argv)
    }

    fn run_with_consensus_full(extra: &[&str]) -> Result<Cli, clap::Error> {
        let mut argv = vec![
            "--taxon",
            "bacteria",
            "--strategy",
            "cautious",
            "-d",
            "db",
            "-t",
            "tax.json",
            "--blast-out-file",
            "blast.out",
        ];
        argv.extend_from_slice(extra);
        run_with_consensus(&argv)
    }

    fn debug_of(cli: &Cli) -> String {
        format!("{:?}", cli.opts)
    }

    fn err_kind(res: Result<Cli, clap::Error>) -> ErrorKind {
        res.expect_err("parse should fail").kind()
    }

    #[test]
    fn cli_command_debug_assert_passes() {
        Cli::command().debug_assert();
    }

    #[test]
    fn cli_command_debug_assert_does_not_evaluate_stdin_defaults() {
        // `query` and `blu_result` both default to "-". If `debug_assert`
        // parsed defaults, `STDIN_HAS_BEEN_USED` would already be set here.
        // This is the only test in this binary allowed to parse "-".
        Cli::command().debug_assert();

        assert!("-".parse::<FileOrStdin>().is_ok());
    }

    #[test]
    fn run_with_consensus_all_required_args_parses() {
        let cli = run_with_consensus_full(&[]).unwrap();
        let debug = debug_of(&cli);

        assert!(debug.starts_with("Blastn("), "{debug}");
        assert!(debug.contains("RunWithConsensus("), "{debug}");
        assert!(debug.contains(r#"source: "query.fna""#), "{debug}");
        assert!(debug.contains(r#"database: "db""#), "{debug}");
        assert!(debug.contains(r#"tax_file: "tax.json""#), "{debug}");
        assert!(debug.contains(r#"blast_out_file: "blast.out""#), "{debug}");
        assert!(debug.contains("taxon: Bacteria"), "{debug}");
        assert!(debug.contains("strategy: Cautious"), "{debug}");
    }

    #[test]
    fn run_with_consensus_missing_each_required_arg_is_rejected() {
        let full = [
            ("--taxon", "bacteria"),
            ("--strategy", "cautious"),
            ("-d", "db"),
            ("-t", "tax.json"),
            ("--blast-out-file", "blast.out"),
        ];

        for skipped in 0..full.len() {
            let argv: Vec<&str> = full
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != skipped)
                .flat_map(|(_, (flag, value))| [*flag, *value])
                .collect();

            assert_eq!(
                err_kind(run_with_consensus(&argv)),
                ErrorKind::MissingRequiredArgument,
                "missing {}",
                full[skipped].0
            );
        }
    }

    #[test]
    fn run_with_consensus_optional_args_have_documented_defaults() {
        let cli = run_with_consensus_full(&[]).unwrap();
        let debug = debug_of(&cli);

        assert!(debug.contains("out_format: Json"), "{debug}");
        assert!(debug.contains("blutils_out_file: None"), "{debug}");
        assert!(debug.contains("custom_taxon_cutoff_file: None"), "{debug}");
        assert!(debug.contains("use_taxid: false"), "{debug}");
        assert!(debug.contains("force_overwrite: false"), "{debug}");
        assert!(debug.contains("max_target_seqs: None"), "{debug}");
        assert!(debug.contains("perc_identity: None"), "{debug}");
        assert!(debug.contains("query_cov: None"), "{debug}");
        assert!(debug.contains("strand: None"), "{debug}");
        assert!(debug.contains("e_value: None"), "{debug}");
        assert!(debug.contains("word_size: None"), "{debug}");
    }

    #[test]
    fn global_threads_defaults_to_some_one() {
        let cli = run_with_consensus_full(&[]).unwrap();

        assert_eq!(cli.threads, Some(1));
        assert!(cli.log_level.is_none());
        assert!(cli.log_file.is_none());
        assert!(matches!(cli.log_format, LogFormat::Ansi));
    }

    #[test]
    fn global_short_t_is_threads_and_subcommand_short_t_is_tax_file() {
        let cli = Cli::try_parse_from([
            "blu",
            "-t",
            "4",
            "blastn",
            "run-with-consensus",
            "query.fna",
            "--taxon",
            "bacteria",
            "--strategy",
            "cautious",
            "-d",
            "db",
            "-t",
            "tax.json",
            "--blast-out-file",
            "blast.out",
        ])
        .unwrap();

        assert_eq!(cli.threads, Some(4));
        assert!(debug_of(&cli).contains(r#"tax_file: "tax.json""#));
    }

    #[test]
    fn global_threads_after_subcommand_is_not_accepted() {
        // `-t` after the subcommand belongs to the subcommand (`tax_file`), and
        // `--threads` is not a global arg, so clap rejects it there.
        assert_eq!(
            err_kind(run_with_consensus_full(&["--threads", "4"])),
            ErrorKind::UnknownArgument
        );
    }

    #[test]
    fn run_with_consensus_short_flags_set_true() {
        let cli = run_with_consensus_full(&["-u", "-f"]).unwrap();
        let debug = debug_of(&cli);

        assert!(debug.contains("use_taxid: true"), "{debug}");
        assert!(debug.contains("force_overwrite: true"), "{debug}");
    }

    #[test]
    fn run_with_consensus_blast_tuning_args_are_parsed() {
        let cli = run_with_consensus_full(&[
            "-m", "5", "-p", "90", "-q", "70", "--strand", "plus", "-e",
            "0.01", "-w", "11",
        ])
        .unwrap();
        let debug = debug_of(&cli);

        assert!(debug.contains("max_target_seqs: Some(5)"), "{debug}");
        assert!(debug.contains("perc_identity: Some(90)"), "{debug}");
        assert!(debug.contains("query_cov: Some(70)"), "{debug}");
        assert!(debug.contains("strand: Some(Plus)"), "{debug}");
        assert!(debug.contains("e_value: Some(0.01)"), "{debug}");
        assert!(debug.contains("word_size: Some(11)"), "{debug}");
    }

    #[test]
    fn taxon_accepts_every_value_enum_name() {
        for (value, variant) in [
            ("fungi", "Fungi"),
            ("bacteria", "Bacteria"),
            ("eukaryotes", "Eukaryotes"),
            ("custom", "Custom"),
        ] {
            let cli = run_with_consensus(&[
                "--taxon",
                value,
                "--strategy",
                "cautious",
                "-d",
                "db",
                "-t",
                "tax.json",
                "--blast-out-file",
                "blast.out",
            ])
            .unwrap();

            assert!(debug_of(&cli).contains(&format!("taxon: {variant}")));
        }
    }

    #[test]
    fn taxon_accepts_short_aliases() {
        // BUG-R37: the short aliases accepted by `Taxon::from_str` must also
        // be accepted by clap, which parses through the `ValueEnum` derive.
        for (value, variant) in [
            ("f", "Fungi"),
            ("b", "Bacteria"),
            ("e", "Eukaryotes"),
            ("c", "Custom"),
        ] {
            let cli = run_with_consensus(&[
                "--taxon",
                value,
                "--strategy",
                "cautious",
                "-d",
                "db",
                "-t",
                "tax.json",
                "--blast-out-file",
                "blast.out",
            ])
            .unwrap();

            assert!(debug_of(&cli).contains(&format!("taxon: {variant}")));
        }
    }

    #[test]
    fn strategy_accepts_only_cautious_and_relaxed() {
        for (value, variant) in
            [("cautious", "Cautious"), ("relaxed", "Relaxed")]
        {
            let cli = run_with_consensus(&[
                "--taxon",
                "bacteria",
                "--strategy",
                value,
                "-d",
                "db",
                "-t",
                "tax.json",
                "--blast-out-file",
                "blast.out",
            ])
            .unwrap();

            assert!(debug_of(&cli).contains(&format!("strategy: {variant}")));
        }

        let res = run_with_consensus(&[
            "--taxon",
            "bacteria",
            "--strategy",
            "strict",
            "-d",
            "db",
            "-t",
            "tax.json",
            "--blast-out-file",
            "blast.out",
        ]);
        assert_eq!(err_kind(res), ErrorKind::InvalidValue);
    }

    #[test]
    fn out_format_accepts_json_jsonl_yaml_only() {
        for (value, variant) in
            [("json", "Json"), ("jsonl", "Jsonl"), ("yaml", "Yaml")]
        {
            let cli =
                run_with_consensus_full(&["--out-format", value]).unwrap();

            assert!(debug_of(&cli).contains(&format!("out_format: {variant}")));
        }

        assert_eq!(
            err_kind(run_with_consensus_full(&["--out-format", "tsv"])),
            ErrorKind::InvalidValue
        );
    }

    #[test]
    fn strand_accepts_both_plus_minus_only() {
        for (value, variant) in
            [("both", "Both"), ("plus", "Plus"), ("minus", "Minus")]
        {
            let cli = run_with_consensus_full(&["--strand", value]).unwrap();

            assert!(
                debug_of(&cli).contains(&format!("strand: Some({variant})"))
            );
        }

        assert_eq!(
            err_kind(run_with_consensus_full(&["--strand", "reverse"])),
            ErrorKind::InvalidValue
        );
    }

    #[test]
    fn log_format_accepts_ansi_and_jsonl_only() {
        let cli = Cli::try_parse_from([
            "blu",
            "--log-format",
            "jsonl",
            "blastn",
            "build-tabular",
            "result.json",
        ])
        .unwrap();
        assert!(matches!(cli.log_format, LogFormat::Jsonl));

        let res = Cli::try_parse_from([
            "blu",
            "--log-format",
            "yaml",
            "blastn",
            "build-tabular",
            "result.json",
        ]);
        assert_eq!(err_kind(res), ErrorKind::InvalidValue);
    }

    #[test]
    fn build_consensus_required_args_and_defaults() {
        let cli = Cli::try_parse_from([
            "blu",
            "blastn",
            "build-consensus",
            "blast.tsv",
            "-t",
            "tax.json",
            "--taxon",
            "fungi",
            "--strategy",
            "relaxed",
        ])
        .unwrap();
        let debug = debug_of(&cli);

        assert!(debug.contains("BuildConsensus("), "{debug}");
        assert!(debug.contains(r#"blast_out: "blast.tsv""#), "{debug}");
        assert!(debug.contains("out_format: Json"), "{debug}");
        assert!(debug.contains("use_taxid: false"), "{debug}");

        let res = Cli::try_parse_from([
            "blu",
            "blastn",
            "build-consensus",
            "blast.tsv",
            "--taxon",
            "fungi",
            "--strategy",
            "relaxed",
        ]);
        assert_eq!(err_kind(res), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn build_tabular_defaults_input_format_json() {
        let cli = Cli::try_parse_from([
            "blu",
            "blastn",
            "build-tabular",
            "result.json",
        ])
        .unwrap();
        let debug = debug_of(&cli);

        assert!(debug.contains(r#"source: "result.json""#), "{debug}");
        assert!(debug.contains("output_file: None"), "{debug}");
        assert!(debug.contains("input_format: Json"), "{debug}");
    }

    #[test]
    fn build_db_blu_without_flags_sets_drop_non_linnaean_to_some_false() {
        // [H] `ArgAction::SetTrue` on an `Option<bool>` yields `Some(false)`
        // when the flag is absent, never `None`.
        let cli = Cli::try_parse_from([
            "blu", "build-db", "blu", "db", "taxdump", "out.json",
        ])
        .unwrap();
        let debug = debug_of(&cli);

        assert!(
            debug.contains("drop_non_linnaean_taxonomies: Some(false)"),
            "{debug}"
        );
        assert!(debug.contains("skip_taxid: None"), "{debug}");
        assert!(debug.contains("replace_rank: None"), "{debug}");
    }

    #[test]
    fn build_db_blu_flags_and_repeated_options_are_collected() {
        let cli = Cli::try_parse_from([
            "blu",
            "build-db",
            "blu",
            "db",
            "taxdump",
            "out.json",
            "-d",
            "-s",
            "131567",
            "-s",
            "2",
            "-r",
            "superkingdom=d",
            "-r",
            "clade=cl",
        ])
        .unwrap();
        let debug = debug_of(&cli);

        assert!(
            debug.contains("drop_non_linnaean_taxonomies: Some(true)"),
            "{debug}"
        );
        assert!(debug.contains("skip_taxid: Some([131567, 2])"), "{debug}");
        assert!(
            debug.contains(
                r#"replace_rank: Some(["superkingdom=d", "clade=cl"])"#
            ),
            "{debug}"
        );
    }

    #[test]
    fn build_db_blu_non_numeric_skip_taxid_is_rejected() {
        let res = Cli::try_parse_from([
            "blu", "build-db", "blu", "db", "taxdump", "out.json", "-s", "abc",
        ]);

        assert_eq!(err_kind(res), ErrorKind::ValueValidation);
    }

    #[test]
    fn build_db_qiime2_and_kraken2_parse_their_args() {
        let cli = Cli::try_parse_from([
            "blu", "build-db", "qiime2", "tax.json", "out_tax", "db",
            "out_seq", "-u",
        ])
        .unwrap();
        let debug = debug_of(&cli);
        assert!(debug.contains("Qiime2("), "{debug}");
        assert!(debug.contains("use_taxid: true"), "{debug}");

        let cli = Cli::try_parse_from([
            "blu", "build-db", "kraken2", "db", "-o", "out_dir",
        ])
        .unwrap();
        let debug = debug_of(&cli);
        assert!(debug.contains("Kraken2("), "{debug}");
        assert!(debug.contains(r#"output_directory: "out_dir""#), "{debug}");

        let res = Cli::try_parse_from(["blu", "build-db", "kraken2", "db"]);
        assert_eq!(err_kind(res), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn check_linux_parses() {
        let cli = Cli::try_parse_from(["blu", "check", "linux"]).unwrap();

        assert!(debug_of(&cli).contains("Linux"));
    }
}
