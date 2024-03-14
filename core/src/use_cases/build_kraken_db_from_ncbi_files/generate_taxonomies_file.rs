use crate::use_cases::shared::write_or_append_to_file;

use mycelium_base::utils::errors::MappedErrors;
use std::{fs::remove_file, path::PathBuf};
use tracing::warn;

pub(super) fn generate_taxonomies_file(
    headers: Vec<(String, usize)>,
    mut output_file: PathBuf,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize the fasta file
    // ? -----------------------------------------------------------------------

    output_file.set_extension("txt");

    if output_file.exists() {
        warn!("Output file already exists. Removing it.");
        remove_file(output_file.to_owned()).unwrap();
    }

    let (file_writer, file) = write_or_append_to_file(&output_file);

    // ? -----------------------------------------------------------------------
    // ? Write the taxonomies file
    // ? -----------------------------------------------------------------------

    headers.iter().for_each(|(header, taxid)| {
        file_writer(
            format!("TAXID\tkraken:taxid|{}|{}\t{}\n", taxid, header, taxid),
            file.try_clone().expect(
                "Unexpected error detected on write taxonomies database",
            ),
        )
        .unwrap();
    });

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(())
}
