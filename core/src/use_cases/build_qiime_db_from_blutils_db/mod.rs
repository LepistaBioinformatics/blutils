use super::shared::{
    append, remove_if_exists, validate_blast_database, write_or_append_to_file,
};
use crate::domain::dtos::taxonomies_map::TaxonomiesMap;

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::{
    fs::{create_dir_all, read_to_string},
    io::{BufRead, BufReader},
    path::PathBuf,
};
use subprocess::Exec;
use tracing::error;

pub fn build_qiime_db_from_blutils_db(
    taxonomies_database_path: &PathBuf,
    mut output_taxonomies_file: PathBuf,
    blast_database_path: &PathBuf,
    mut output_sequences_file: PathBuf,
    use_taxid: Option<bool>,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Parse the taxonomies database
    // ? -----------------------------------------------------------------------

    output_taxonomies_file.set_extension("tsv");
    output_sequences_file.set_extension("fna");

    for output_file in [&output_taxonomies_file, &output_sequences_file] {
        if let Some(parent) = output_file.parent() {
            if !parent.as_os_str().is_empty() {
                create_dir_all(parent).map_err(|err| {
                    execution_err(format!(
                        "Could not create output directory {parent:?}: {err}"
                    ))
                })?;
            }
        }
    }

    // BUG-R43: file-system errors return `Err` instead of panicking.
    remove_if_exists(&output_taxonomies_file)?;

    let taxonomies_database_content = read_to_string(taxonomies_database_path)
        .map_err(|err| {
            execution_err(format!(
                "Could not read the taxonomies database {taxonomies_database_path:?}: {err}"
            ))
        })?;

    let taxonomy_map = match serde_json::from_str::<TaxonomiesMap>(
        &taxonomies_database_content,
    ) {
        Err(err) => {
            error!("Unexpected error detected on read `taxonomies`: {}", err);
            return execution_err(String::from(
                "Unexpected error occurred on load table.",
            ))
            .as_error();
        }
        Ok(res) => res,
    };

    let (tax_writer, tax_file) =
        write_or_append_to_file(&output_taxonomies_file)?;

    append(tax_writer, &tax_file, format!("Feature ID\tTaxon\n"))?;

    //
    // Write the output file
    //
    taxonomy_map
        .taxonomies
        .iter()
        .flat_map(|record| {
            record.accessions.iter().map(move |accession| {
                format!(
                    "{}-{}-{}\t{}\n",
                    record.taxid,
                    accession.oid,
                    accession.accession,
                    if let Some(true) = use_taxid {
                        record.numeric_lineage.to_owned()
                    } else {
                        record.text_lineage.to_owned()
                    }
                )
            })
        })
        .try_for_each(|line| append(tax_writer, &tax_file, line))?;

    // ? -----------------------------------------------------------------------
    // ? Validate and parse the blast database
    // ? -----------------------------------------------------------------------

    let er_msg = "Invalid line detected on blastdbcmd response";

    validate_blast_database(blast_database_path)?;

    output_sequences_file.set_extension("fna");

    remove_if_exists(&output_sequences_file)?;

    let (fna_writer, fna_file) =
        write_or_append_to_file(&output_sequences_file)?;

    match Exec::cmd("blastdbcmd")
        .arg("-entry")
        .arg("all")
        .arg("-db")
        .arg(blast_database_path)
        .arg("-outfmt")
        .arg("%a  %T  %o  %s")
        .stream_stdout()
    {
        Err(err) => {
            return execution_err(format!(
                "Unexpected error detected on execute blastdbcmd: {err}"
            ))
            .as_error()
        }
        Ok(res) => {
            let mut stream = BufReader::new(res);
            let mut buf_line = String::new();

            while let Ok(bites) = stream.read_line(&mut buf_line) {
                if bites == 0 {
                    break;
                }

                let mut line = buf_line.split("  ");
                let mut field = || {
                    line.next().map(str::trim).ok_or_else(|| {
                        execution_err(format!(
                            "{er_msg}: {:?}",
                            buf_line.trim_end()
                        ))
                    })
                };

                // BUG-R43: a short `blastdbcmd` line returns `Err`.
                let (accession, taxid, sequence_hash, sequence) =
                    (field()?, field()?, field()?, field()?);

                append(
                    fna_writer,
                    &fna_file,
                    format!(
                        ">{taxid}-{sequence_hash}-{accession}\n{sequence}\n"
                    ),
                )?;

                buf_line.clear();
            }
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(())
}
