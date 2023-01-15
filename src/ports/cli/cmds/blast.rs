use blul::{
    adapters::proc::execute_step::ExecuteStepProcRepository,
    domain::dtos::blast_builder::{BlastBuilder, Taxon},
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

    match run_blast_and_build_consensus(
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
        Ok(_) => (),
    };
}
