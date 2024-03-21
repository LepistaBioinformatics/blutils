pub(crate) use blul_core::domain::dtos::{
    blast_builder::Strand, consensus_strategy::ConsensusStrategy, taxon::Taxon,
};

use blul_core::{
    domain::dtos::file_or_stdin::FileOrStdin, use_cases::OutputFormat,
};
use clap::Parser;
use std::path::PathBuf;

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

    /// Build tabular output.
    BuildTabular(BuildTabularArguments),
}

#[derive(Parser, Debug)]
pub(crate) struct RunBlastAndBuildConsensusArguments {
    /// If the value is "-", the STDIN will be used and this command will expect
    /// to receive the blutils output from the STDIN.
    #[clap(default_value = "-")]
    pub(super) query: FileOrStdin,

    /// The reference sequences system file path
    #[arg(short, long)]
    pub(super) database: String,

    /// The taxonomy system file path
    #[arg(short, long)]
    pub(super) tax_file: String,

    /// The output directory
    #[arg(long)]
    pub(super) blast_out_file: String,

    /// The output file
    #[arg(long)]
    pub(super) blutils_out_file: Option<String>,

    /// The output file format
    #[arg(long, default_value = "json")]
    pub(super) out_format: OutputFormat,

    /// This option checks the higher taxon which the consensus search should be
    /// based
    #[arg(long)]
    pub(super) taxon: Taxon,

    #[arg(long, short)]
    pub(super) custom_taxon_cutoff_file: Option<PathBuf>,

    /// The strategy to be used
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
    #[arg(long)]
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
    #[arg(short, long)]
    pub(super) tax_file: String,

    /// The output file
    #[arg(long)]
    pub(super) blutils_out_file: Option<String>,

    /// This option checks the higher taxon which the consensus search should be
    /// based
    #[arg(long)]
    pub(super) taxon: Taxon,

    #[arg(long, short)]
    pub(super) custom_taxon_cutoff_file: Option<PathBuf>,

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

    /// The output file format
    #[arg(long, default_value = "json")]
    pub(super) out_format: OutputFormat,
}

#[derive(Parser, Debug)]
pub(crate) struct BuildTabularArguments {
    /// The blutils output file path or STDIN
    ///
    /// If the value is "-", the STDIN will be used and this command will expect
    /// to receive the blutils output from the STDIN.
    #[clap(default_value = "-")]
    pub(super) blu_result: FileOrStdin,

    /// The tabular output file
    #[arg(short, long)]
    pub(super) output_file: Option<String>,

    /// The input file format
    #[arg(short, long, default_value = "json")]
    pub(super) input_format: OutputFormat,
}
