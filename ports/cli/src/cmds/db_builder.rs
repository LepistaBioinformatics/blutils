use blul_core::use_cases::{
    build_ref_db_from_ncbi_files, check_host_requirements,
};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub(crate) struct Arguments {
    #[clap(subcommand)]
    pub build: Commands,
}

#[derive(Parser, Debug)]
pub(crate) enum Commands {
    /// Run blast and generate consensus identities.
    BlutilsDatabase(BuildDatabaseArguments),
}

#[derive(Parser, Debug)]
pub(crate) struct BuildDatabaseArguments {
    blast_database_path: String,

    taxdump_directory_path: PathBuf,

    #[arg(short, long)]
    ignore_taxids: Option<Vec<u64>>,

    /// The number of threads to be used. Default is 1.
    #[arg(short, long)]
    threads: Option<usize>,
}

pub(crate) fn run_blast_and_build_consensus_cmd(args: BuildDatabaseArguments) {
    // Execute system checks before running the blast
    if let Err(err) = check_host_requirements(Some("debug")) {
        panic!("{err}");
    }

    let threads = match args.threads {
        Some(n) => n,
        None => 1,
    };

    match build_ref_db_from_ncbi_files(
        &args.blast_database_path,
        args.taxdump_directory_path,
        args.ignore_taxids,
        threads,
    ) {
        Err(err) => panic!("{err}"),
        Ok(_) => (),
    };
}
