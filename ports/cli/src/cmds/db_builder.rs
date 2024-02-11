use blul_core::use_cases::{
    build_ref_db_from_ncbi_files, check_host_requirements,
};
use clap::{ArgAction, Parser};
use core::panic;
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
    /// The path to the blast database
    ///
    /// The path to the blast database that will be used to build the consensus
    /// taxonomy. The blast database should be a nucleotide database. The
    /// database should be created using the makeblastdb command from the blast
    /// package.
    ///
    blast_database_path: String,

    /// The path to the taxdump directory
    ///
    /// The path to the taxdump directory that contains the NCBI taxonomy
    /// database. The taxdump directory should be downloaded from the NCBI
    /// taxonomy database. The taxdump should be downloaded from the NCBI and
    /// uncompressed.
    ///
    taxdump_directory_path: PathBuf,

    /// Drop non Linnaean taxonomies
    ///
    /// If this option is set, the non Linnaean taxonomies will be dropped from
    /// the taxonomy building process. The non Linnaean taxonomies are the ones
    /// that are not part of the Linnaean taxonomy system. The default value is
    /// false.
    ///
    #[arg(short, long, action=ArgAction::SetTrue)]
    drop_non_linnaean_taxonomies: Option<bool>,

    /// Specify taxids to be skipped
    /// Example: --skip-taxid 131567
    ///
    /// The specified taxid will be skipped in the taxonomy building process. It
    /// should be used to skip multiple taxids.
    /// Example: --skip-taxid 131567 --skip-taxid 2
    ///
    #[arg(short, long)]
    skip_taxid: Option<Vec<u64>>,

    /// Replace a rank by another
    /// Example: --replace-rank 'superkingdom=d'. It is common to use this
    /// option to replace the superkingdom rank by domain in bacterial taxonomy.
    ///
    /// Multiple ranks can be replaced by using the option multiple times.
    /// Example: --replace-rank 'superkingdom=d' --replace-rank 'clade=cl'
    ///
    #[arg(short, long)]
    replace_rank: Option<Vec<String>>,
}

pub(crate) fn run_blast_and_build_consensus_cmd(args: BuildDatabaseArguments) {
    // Execute system checks before running the blast
    if let Err(err) = check_host_requirements(Some("debug")) {
        panic!("{err}");
    }

    match build_ref_db_from_ncbi_files(
        &args.blast_database_path,
        args.taxdump_directory_path,
        args.skip_taxid,
        match args.replace_rank {
            Some(ranks) => {
                let mut replace_rank = std::collections::HashMap::new();
                for rank in ranks {
                    let splitted: Vec<&str> = rank.split("=").collect();
                    if splitted.len() != 2 {
                        panic!("Invalid replace rank option: {:?}", rank);
                    }
                    replace_rank
                        .insert(splitted[0].to_owned(), splitted[1].to_owned());
                }
                Some(replace_rank)
            }
            None => None,
        },
        args.drop_non_linnaean_taxonomies,
    ) {
        Err(err) => panic!("{err}"),
        Ok(_) => (),
    };
}
