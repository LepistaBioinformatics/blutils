mod build_fasta_database;
mod build_taxonomy_database;

use self::build_taxonomy_database::build_taxonomy_database;
use build_fasta_database::build_fasta_database;
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::{collections::HashMap, path::PathBuf};
use tracing::info;

/// Build blutil sreference database from NCBI files
#[tracing::instrument(name = "Build Reference DB from New TaxDump")]
pub fn build_ref_db_from_ncbi_files(
    blast_database_path: &str,
    taxdump_directory_path: PathBuf,
    ignore_taxids: Option<Vec<u64>>,
    replace_rank: Option<HashMap<String, String>>,
    threads: usize,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Build blast database
    // ? -----------------------------------------------------------------------

    info!("Building blast database from: {:?}", blast_database_path);

    let (output_path, accessions_map) =
        build_fasta_database(blast_database_path)?;

    info!("Blast database built successfully");

    // ? -----------------------------------------------------------------------
    // ? Build taxonomy database from tax-ids
    // ? -----------------------------------------------------------------------

    if !taxdump_directory_path.is_dir() {
        return execution_err(format!(
            "Invalid taxdump directory path: {:?}",
            taxdump_directory_path
        ))
        .as_error();
    }

    build_taxonomy_database(
        taxdump_directory_path.join("names.dmp"),
        taxdump_directory_path.join("nodes.dmp"),
        taxdump_directory_path.join("taxidlineage.dmp"),
        taxdump_directory_path.join("delnodes.dmp"),
        taxdump_directory_path.join("merged.dmp"),
        accessions_map,
        ignore_taxids,
        replace_rank,
        output_path,
        threads,
    )?;

    Ok(())
}
