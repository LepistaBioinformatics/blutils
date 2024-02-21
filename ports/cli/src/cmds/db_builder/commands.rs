use clap::{ArgAction, Parser};
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub(crate) struct Arguments {
    #[clap(subcommand)]
    pub build: Commands,
}

#[derive(Parser, Debug)]
pub(crate) enum Commands {
    /// Build the Blutils database.
    Blu(BuildBlutilsDatabaseArguments),

    /// Build QIIME database from the Blutils database.
    Qiime(BuildQiimeDatabaseArguments),
}

#[derive(Parser, Debug)]
pub(crate) struct BuildBlutilsDatabaseArguments {
    /// The path to the blast database
    ///
    /// The path to the blast database that will be used to build the consensus
    /// taxonomy. The blast database should be a nucleotide database. The
    /// database should be created using the makeblastdb command from the blast
    /// package.
    ///
    pub(super) blast_database_path: String,

    /// The path to the taxdump directory
    ///
    /// The path to the taxdump directory that contains the NCBI taxonomy
    /// database. The taxdump directory should be downloaded from the NCBI
    /// taxonomy database. The taxdump should be downloaded from the NCBI and
    /// uncompressed.
    ///
    pub(super) taxdump_directory_path: PathBuf,

    /// The path where the output file will be saved
    ///
    /// The output file is a JSON file that contains the taxonomies database.
    pub(super) output_file_path: PathBuf,

    /// Drop non Linnaean taxonomies
    ///
    /// If this option is set, the non Linnaean taxonomies will be dropped from
    /// the taxonomy building process. The non Linnaean taxonomies are the ones
    /// that are not part of the Linnaean taxonomy system. The default value is
    /// false.
    ///
    #[arg(short, long, action=ArgAction::SetTrue)]
    pub(super) drop_non_linnaean_taxonomies: Option<bool>,

    /// Specify taxids to be skipped
    /// Example: --skip-taxid 131567
    ///
    /// The specified taxid will be skipped in the taxonomy building process. It
    /// should be used to skip multiple taxids.
    /// Example: --skip-taxid 131567 --skip-taxid 2
    ///
    #[arg(short, long)]
    pub(super) skip_taxid: Option<Vec<u64>>,

    /// Replace a rank by another
    /// Example: --replace-rank 'superkingdom=d'. It is common to use this
    /// option to replace the superkingdom rank by domain in bacterial taxonomy.
    ///
    /// Multiple ranks can be replaced by using the option multiple times.
    /// Example: --replace-rank 'superkingdom=d' --replace-rank 'clade=cl'
    ///
    #[arg(short, long)]
    pub(super) replace_rank: Option<Vec<String>>,
}

#[derive(Parser, Debug)]
pub(crate) struct BuildQiimeDatabaseArguments {
    /// The path to the blutils taxonomy database
    pub(super) taxonomies_database_path: PathBuf,

    /// The path to the QIIME output taxonomies file
    pub(super) output_taxonomies_file: PathBuf,

    /// The path to the blast database
    pub(super) blast_database_path: PathBuf,

    /// The path to the QIIME output sequences file
    pub(super) output_sequences_file: PathBuf,

    /// Use taxid instead of taxonomy
    ///
    /// If true, the consensus will be based on the taxid instead of the
    /// taxonomy itself.
    #[arg(short, long, default_value = "false")]
    pub(super) use_taxid: bool,
}
