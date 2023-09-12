mod build_fasta_database;
mod build_taxonomy_database;

use build_fasta_database::build_fasta_database;
use std::path::PathBuf;

use clean_base::utils::errors::{factories::execution_err, MappedErrors};

use self::build_taxonomy_database::build_taxonomy_database;

/// Build blutil sreference database from NCBI files
pub fn build_ref_db_from_ncbi_files(
    blast_database_path: &str,
    taxdump_directory_path: PathBuf,
    threads: usize,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Build blast database
    // ? -----------------------------------------------------------------------

    let (_, _) = build_fasta_database(blast_database_path)?;

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

    let names = taxdump_directory_path.join("names.dmp");
    let nodes = taxdump_directory_path.join("nodes.dmp");
    let lineage = taxdump_directory_path.join("taxidlineage.dmp");
    let reference_taxonomy_df =
        build_taxonomy_database(names, nodes, lineage, threads)?;

    println!("reference_taxonomy_df:\n{:?}", reference_taxonomy_df);

    Ok(())
}
