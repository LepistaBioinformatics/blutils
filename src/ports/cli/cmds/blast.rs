use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use blul::{
    adapters::proc::execute_step::ExecuteStepProcRepository,
    domain::dtos::{
        blast_builder::{BlastBuilder, Taxon},
        blast_result::{BlastQueryConsensusResult, ConsensusResult},
    },
    use_cases::{run_blast_and_build_consensus, ConsensusStrategy},
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

    #[arg(long)]
    taxon: Taxon,

    #[arg(long)]
    strategy: ConsensusStrategy,

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
    let config = BlastBuilder::create(&args.subject, args.taxon);

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
        args.strategy,
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
    let mut file = match File::create(out_dir.join("blutils.consensus.json")) {
        Err(err) => panic!("{err}"),
        Ok(res) => res,
    };

    let consensus_type_results = results.iter().fold(
        Vec::<BlastQueryConsensusResult>::new(),
        |mut init, record| {
            match record {
                ConsensusResult::NoConsensusFound(res) => {
                    init.push(BlastQueryConsensusResult {
                        query: res.query.to_owned(),
                        taxon: None,
                    });
                }
                ConsensusResult::ConsensusFound(res) => {
                    init.push(BlastQueryConsensusResult {
                        query: res.query.to_owned(),
                        taxon: res.taxon.to_owned(),
                    })
                }
            };

            init
        },
    );

    match file.write_all(
        serde_json::to_string_pretty(&consensus_type_results)
            .unwrap()
            .as_bytes(),
    ) {
        Err(err) => panic!("{err}"),
        Ok(_) => (),
    };
}
