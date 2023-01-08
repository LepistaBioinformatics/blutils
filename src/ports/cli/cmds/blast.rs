use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use blul::{
    adapters::proc::execute_step::ExecuteStepProcRepository,
    domain::dtos::{
        blast_builder::BlastBuilder,
        blast_result::{
            BlastQueryConsensusResult, BlastQueryNoConsensusResult,
            ConsensusResult,
        },
    },
    use_cases::run_blast_and_build_consensus,
};
use clap::Parser;

#[derive(Parser, Debug)]
pub(crate) struct Arguments {
    #[clap(subcommand)]
    pub run_blast: Commands,
}

#[derive(Parser, Debug)]
pub(crate) enum Commands {
    RunWithConsensus(RunBlastAndBuildConsensusArguments),
}

#[derive(Parser, Debug)]
pub(crate) struct RunBlastAndBuildConsensusArguments {
    query: String,
    subject: String,
    tax_file: String,
    out_dir: String,

    /// Case true, overwrite the output file if exists. Otherwise dispatch an
    /// error if the output file exists.
    #[arg(short, long, default_value = "false")]
    force_overwrite: bool,

    /// The number of threads to be used. Default is 1.
    #[arg(short, long)]
    threads: Option<usize>,
}

pub(crate) fn run_blast_and_build_consensus_cmd(
    args: RunBlastAndBuildConsensusArguments,
) {
    let repo = ExecuteStepProcRepository {};

    // Create configuration DTO
    let config = BlastBuilder::create(&args.subject);

    // Set the default number of threads
    let threads = match args.threads {
        Some(n) => n,
        None => 1,
    };

    // Run Blast
    let blast_output = match run_blast_and_build_consensus(
        &args.query,
        &args.tax_file,
        &args.out_dir,
        config,
        &repo,
        &args.force_overwrite,
        threads,
    ) {
        Err(err) => panic!("{err}"),
        Ok(res) => res,
    };

    write_json_output(
        blast_output.to_owned(),
        Path::new(&args.out_dir).to_path_buf(),
    )
}

fn write_json_output(results: Vec<ConsensusResult>, out_dir: PathBuf) {
    //
    // Collect and persist consensus sequences
    //
    let match_seqs = results
        .iter()
        .filter_map(|i| match i {
            ConsensusResult::ConsensusFound(res) => Some(res.to_owned()),
            _ => None,
        })
        .collect::<Vec<BlastQueryConsensusResult>>();

    write_json_file::<BlastQueryConsensusResult>(
        match_seqs,
        out_dir.join("consensus-match.json"),
    );

    //
    // Collect and persist no-consensus sequences
    //
    let unmatch_seqs = results
        .iter()
        .filter_map(|i| match i {
            ConsensusResult::NoConsensusFound(res) => Some(res.to_owned()),
            _ => None,
        })
        .collect::<Vec<BlastQueryNoConsensusResult>>();

    write_json_file::<BlastQueryNoConsensusResult>(
        unmatch_seqs,
        out_dir.join("consensus-unmatch.json"),
    );
}

fn write_json_file<T: serde::ser::Serialize>(records: Vec<T>, file: PathBuf) {
    let mut file = match File::create(file) {
        Err(err) => panic!("{err}"),
        Ok(res) => res,
    };

    match file.write_all(serde_json::to_string(&records).unwrap().as_bytes()) {
        Err(err) => panic!("{err}"),
        Ok(_) => (),
    };
}
