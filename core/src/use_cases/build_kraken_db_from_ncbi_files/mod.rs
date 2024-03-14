mod generate_fasta_file;
mod generate_taxonomies_file;

use generate_fasta_file::*;
use generate_taxonomies_file::*;

use mycelium_base::utils::errors::MappedErrors;
use std::{
    fs::{create_dir_all, remove_file},
    path::PathBuf,
};
use tracing::warn;

pub fn build_kraken_db_from_ncbi_files(
    blast_database_path: &PathBuf,
    output_directory: PathBuf,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize files
    // ? -----------------------------------------------------------------------

    if output_directory.exists() {
        warn!("Output directory already exists. Removing it.");
        remove_file(output_directory.to_owned()).unwrap();
    }

    if !output_directory.is_dir() {
        create_dir_all(&output_directory).unwrap();
    }

    let output_sequences_file = output_directory.join("sequences.fna");
    let output_taxonomies_file = output_directory.join("taxonomies.tsv");

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
