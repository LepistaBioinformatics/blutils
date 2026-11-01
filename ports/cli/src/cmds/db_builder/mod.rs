mod commands;

use self::commands::BuildKraken2DatabaseArguments;

use anyhow::{anyhow, bail, Result};
use blul_core::use_cases::{
    build_kraken_db_from_ncbi_files, build_qiime_db_from_blutils_db,
    build_ref_db_from_ncbi_files, check_host_requirements,
};
use commands::BuildQiimeDatabaseArguments;
pub(crate) use commands::{Arguments, BuildBlutilsDatabaseArguments, Commands};

pub(crate) fn run_blast_and_build_consensus_cmd(
    args: BuildBlutilsDatabaseArguments,
) -> Result<()> {
    // Execute system checks before running the blast
    check_host_requirements(Some("debug")).map_err(|err| anyhow!("{err}"))?;

    build_ref_db_from_ncbi_files(
        &args.blast_database_path,
        args.taxdump_directory_path,
        args.skip_taxid,
        match args.replace_rank {
            Some(ranks) => {
                let mut replace_rank = std::collections::HashMap::new();
                for rank in ranks {
                    let splitted: Vec<&str> = rank.split("=").collect();
                    if splitted.len() != 2 {
                        bail!(
                            "Invalid replace rank option: {rank:?} (expected `<rank>=<replacement>`)"
                        );
                    }
                    replace_rank
                        .insert(splitted[0].to_owned(), splitted[1].to_owned());
                }
                Some(replace_rank)
            }
            None => None,
        },
        args.drop_non_linnaean_taxonomies,
        args.output_file_path,
    )
    .map_err(|err| anyhow!("{err}"))
}

pub(crate) fn build_qiime_db_from_blutils_db_cmd(
    args: BuildQiimeDatabaseArguments,
) -> Result<()> {
    // Execute system checks before running the blast
    check_host_requirements(Some("debug")).map_err(|err| anyhow!("{err}"))?;

    build_qiime_db_from_blutils_db(
        &args.taxonomies_database_path,
        args.output_taxonomies_file,
        &args.blast_database_path,
        args.output_sequences_file,
        Some(args.use_taxid),
    )
    .map_err(|err| anyhow!("{err}"))
}

pub(crate) fn build_kraken_db_from_ncbi_files_cmd(
    args: BuildKraken2DatabaseArguments,
) -> Result<()> {
    // Execute system checks before running the blast
    check_host_requirements(Some("debug")).map_err(|err| anyhow!("{err}"))?;

    build_kraken_db_from_ncbi_files(
        &args.blast_database_path,
        args.output_directory,
    )
    .map_err(|err| anyhow!("{err}"))
}
