mod build_accessions_map;
mod build_taxonomy_database;
mod load_del_nodes_dataframe;
mod load_dump_file;
mod load_lineage_dataframe;
mod load_merged_dataframe;
mod load_names_dataframe;
mod load_nodes_dataframe;

use build_accessions_map::*;
use build_taxonomy_database::*;
use load_del_nodes_dataframe::*;
use load_dump_file::*;
use load_lineage_dataframe::*;
use load_merged_dataframe::*;
use load_names_dataframe::*;
use load_nodes_dataframe::*;

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::{collections::HashMap, path::PathBuf};
use tracing::info;

/// Build blutil sreference database from NCBI files
#[tracing::instrument(
    name = "Build Reference DB from New TaxDump",
    skip(
        blast_database_path,
        taxdump_directory_path,
        ignore_taxids,
        replace_rank,
        drop_non_linnaean_taxonomies
    )
)]
pub fn build_ref_db_from_ncbi_files(
    blast_database_path: &str,
    taxdump_directory_path: PathBuf,
    ignore_taxids: Option<Vec<u64>>,
    replace_rank: Option<HashMap<String, String>>,
    drop_non_linnaean_taxonomies: Option<bool>,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Build blast database
    // ? -----------------------------------------------------------------------

    info!("Collecting base taxonomies for {:?}", blast_database_path);

    let taxids_map = build_accessions_map(blast_database_path)?;

    info!("Base taxonomies collected successfully");

    // ? -----------------------------------------------------------------------
    // ? Build taxonomy database from tax-ids
    // ? -----------------------------------------------------------------------

    info!(
        "Building taxonomy database from: {:?}",
        taxdump_directory_path
    );

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
        taxids_map,
        ignore_taxids,
        replace_rank,
        drop_non_linnaean_taxonomies,
        blast_database_path.to_string(),
    )?;

    info!("Taxonomy database built successfully");

    Ok(())
}
