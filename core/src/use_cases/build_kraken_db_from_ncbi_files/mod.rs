mod generate_fasta_file;
mod generate_taxonomies_file;

use super::shared::validate_blast_database;
use generate_fasta_file::*;
use generate_taxonomies_file::*;

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::{
    fs::{create_dir_all, remove_dir_all, remove_file},
    path::PathBuf,
};
use tracing::warn;

pub fn build_kraken_db_from_ncbi_files(
    blast_database_path: &PathBuf,
    output_directory: PathBuf,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Validate inputs before touching the output directory
    // ? -----------------------------------------------------------------------

    validate_blast_database(blast_database_path)?;

    // ? -----------------------------------------------------------------------
    // ? Initialize files
    // ? -----------------------------------------------------------------------

    if output_directory.exists() {
        warn!("Output directory already exists. Removing it.");

        // BUG-R43: file-system errors return `Err` instead of panicking.
        let removed = if output_directory.is_dir() {
            remove_dir_all(&output_directory)
        } else {
            remove_file(&output_directory)
        };

        removed.map_err(|err| {
            execution_err(format!(
                "Could not replace the output directory {output_directory:?}: {err}"
            ))
        })?;
    }

    if !output_directory.is_dir() {
        create_dir_all(&output_directory).map_err(|err| {
            execution_err(format!(
                "Could not create the output directory {output_directory:?}: {err}"
            ))
        })?;
    }

    let output_sequences_file = output_directory.join("library.fna");
    let output_taxonomies_file = output_directory.join("prelim_map.txt");

    // ? -----------------------------------------------------------------------
    // ? Generate sequences database
    // ? -----------------------------------------------------------------------

    let headers =
        generate_fasta_file(blast_database_path, output_sequences_file)?;

    // ? -----------------------------------------------------------------------
    // ? Generate taxonomies database
    // ? -----------------------------------------------------------------------

    generate_taxonomies_file(headers, output_taxonomies_file)?;

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(())
}
