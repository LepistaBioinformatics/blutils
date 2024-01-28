use blul_core::{
    domain::dtos::blast_builder::{BlastBuilder, Strand, Taxon},
    use_cases::{
        check_host_requirements, run_blast_and_build_consensus,
        ConsensusStrategy,
    },
};
use blul_proc::execute_blast::ExecuteBlastnProcRepository;
use clap::Parser;

#[derive(Parser, Debug)]
pub(crate) struct Arguments {
    #[clap(subcommand)]
    pub run_blast: Commands,
}

#[derive(Parser, Debug)]
pub(crate) enum Commands {
    /// Run blast and generate consensus identities.
    RunWithConsensus(RunBlastAndBuildConsensusArguments),
}

#[derive(Parser, Debug)]
pub(crate) struct RunBlastAndBuildConsensusArguments {
    /// The query sequences system file path
    query: String,

    /// The reference sequences system file path
    subject: String,

    /// The taxonomy system file path
    tax_file: String,

    /// The output directory
    out_dir: String,

    /// This option checks the higher taxon which the consensus search should be
    /// based
    #[arg(long)]
    taxon: Taxon,

    /// The strategy to be used
    ///
    /// cautious: Select the shortest taxonomic path to find consensus from.
    /// relaxed: Select the longest taxonomic path to find consensus from.
    #[arg(long)]
    strategy: ConsensusStrategy,

    /// Case true, overwrite the output file if exists. Otherwise dispatch an
    /// error if the output file exists.
    #[arg(short, long, default_value = "false")]
    force_overwrite: bool,

    /// The number of threads to be used. Default is 1.
    #[arg(short, long)]
    threads: Option<usize>,

    /// The max target sequences to be used. Default is 10.
    #[arg(short, long)]
    max_target_seqs: Option<i32>,

    /// The percentage of identity to be used. Default is 80.
    #[arg(short, long)]
    perc_identity: Option<i32>,

    /// The query coverage to be used. Default is 80.
    #[arg(short, long)]
    query_cov: Option<i32>,

    /// The strand to be used. Default is both.
    #[arg(long)]
    strand: Option<Strand>,

    /// The e-value to be used. Default is 0.001.
    #[arg(short, long)]
    e_value: Option<f32>,

    /// The word size to be used. Default is 15.
    #[arg(short, long)]
    word_size: Option<i32>,
}

pub(crate) fn run_blast_and_build_consensus_cmd(
    args: RunBlastAndBuildConsensusArguments,
) {
    // Execute system checks before running the blast
    check_host_requirements(Some("debug"));

    let repo = ExecuteBlastnProcRepository {};

    // Create configuration DTO
    let mut config = BlastBuilder::default(&args.subject, args.taxon);

    if args.max_target_seqs.is_some() {
        config = config.with_max_target_seqs(args.max_target_seqs.unwrap());
    }

    if args.perc_identity.is_some() {
        config = config.with_perc_identity(args.perc_identity.unwrap());
    }

    if args.query_cov.is_some() {
        config = config.with_query_cov(args.query_cov.unwrap());
    }

    if args.strand.is_some() {
        config = config.with_strand(args.strand.unwrap());
    }

    if args.e_value.is_some() {
        config = config.with_e_value(args.e_value.unwrap());
    }

    if args.word_size.is_some() {
        config = config.with_word_size(args.word_size.unwrap());
    }

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
