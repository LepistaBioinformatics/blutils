pub(crate) use blul_core::domain::dtos::{
    blast_builder::{Strand, Taxon},
    consensus_strategy::ConsensusStrategy,
};
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

    /// Generate consensus from blast results.
    BuildConsensus(BuildConsensusArguments),
}

#[derive(Parser, Debug)]
pub(crate) struct RunBlastAndBuildConsensusArguments {
    /// The query sequences system file path
    pub(super) query: String,

    /// The reference sequences system file path
    pub(super) database: String,

    /// The taxonomy system file path
    pub(super) tax_file: String,

    /// The output directory
    pub(super) out_dir: String,

    /// This option checks the higher taxon which the consensus search should be
    /// based
    #[arg(long)]
    pub(super) taxon: Taxon,

    /// The strategy to be used
    ///
    /// cautious: Select the shortest taxonomic path to find consensus from.
    /// relaxed: Select the longest taxonomic path to find consensus from.
    #[arg(long)]
    pub(super) strategy: ConsensusStrategy,

    /// Use taxid instead of taxonomy
    ///
    /// If true, the consensus will be based on the taxid instead of the
    /// taxonomy itself.
    #[arg(short, long, default_value = "false")]
    pub(super) use_taxid: bool,

    /// Case true, overwrite the output file if exists. Otherwise dispatch an
    /// error if the output file exists.
    #[arg(short, long, default_value = "false")]
    pub(super) force_overwrite: bool,

    /// The number of threads to be used. Default is 1.
    #[arg(short, long)]
    pub(super) threads: Option<usize>,

    /// The max target sequences to be used. Default is 10.
    #[arg(short, long)]
    pub(super) max_target_seqs: Option<i32>,

    /// The percentage of identity to be used. Default is 80.
    #[arg(short, long)]
    pub(super) perc_identity: Option<i32>,

    /// The query coverage to be used. Default is 80.
    #[arg(short, long)]
    pub(super) query_cov: Option<i32>,

    /// The strand to be used. Default is both.
    #[arg(long)]
    pub(super) strand: Option<Strand>,

    /// The e-value to be used. Default is 0.001.
    #[arg(short, long)]
    pub(super) e_value: Option<f32>,

    /// The word size to be used. Default is 15.
    #[arg(short, long)]
    pub(super) word_size: Option<i32>,
}

#[derive(Parser, Debug)]
pub(crate) struct BuildConsensusArguments {
    /// The reference sequences system file path
    pub(super) blast_out: String,

    /// The taxonomy system file path
    pub(super) tax_file: String,

    /// The output directory
    pub(super) out_dir: String,

    /// This option checks the higher taxon which the consensus search should be
    /// based
    #[arg(long)]
    pub(super) taxon: Taxon,

    /// The strategy to be used
    ///
    /// cautious: Select the shortest taxonomic path to find consensus from.
    /// relaxed: Select the longest taxonomic path to find consensus from.
    #[arg(long)]
    pub(super) strategy: ConsensusStrategy,

    /// Use taxid instead of taxonomy
    ///
    /// If true, the consensus will be based on the taxid instead of the
    /// taxonomy itself.
    #[arg(short, long, default_value = "false")]
    pub(super) use_taxid: bool,
}
