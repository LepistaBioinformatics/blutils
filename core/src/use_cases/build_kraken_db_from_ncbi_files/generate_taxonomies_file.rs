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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{read_to_string, write};

    #[test]
    fn generate_taxonomies_file_any_extension_forces_txt() {
        let dir = tempfile::tempdir().unwrap();

        generate_taxonomies_file(
            vec![("NR_1.1".to_string(), 562)],
            dir.path().join("prelim_map.map"),
        )
        .unwrap();

        assert!(dir.path().join("prelim_map.txt").is_file());
        assert!(!dir.path().join("prelim_map.map").exists());
    }

    #[test]
    fn generate_taxonomies_file_headers_write_kraken_lines_in_input_order() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("prelim_map.txt");

        generate_taxonomies_file(
            vec![("NR_2.1".to_string(), 1423), ("NR_1.1".to_string(), 562)],
            output.clone(),
        )
        .unwrap();

        assert_eq!(
            read_to_string(output).unwrap(),
            "TAXID\tkraken:taxid|1423|NR_2.1\t1423\n\
             TAXID\tkraken:taxid|562|NR_1.1\t562\n"
        );
    }

    #[test]
    fn generate_taxonomies_file_empty_headers_creates_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("prelim_map.txt");

        generate_taxonomies_file(Vec::new(), output.clone()).unwrap();

        assert_eq!(read_to_string(output).unwrap(), "");
    }

    #[test]
    fn generate_taxonomies_file_existing_file_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("prelim_map.txt");
        write(&output, "stale content\n").unwrap();

        generate_taxonomies_file(
            vec![("NR_1.1".to_string(), 562)],
            output.clone(),
        )
        .unwrap();

        assert_eq!(
            read_to_string(output).unwrap(),
            "TAXID\tkraken:taxid|562|NR_1.1\t562\n"
        );
    }
}
