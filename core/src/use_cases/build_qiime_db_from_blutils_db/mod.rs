use super::shared::{validate_blast_database, write_or_append_to_file};
use crate::domain::dtos::taxonomies_map::TaxonomiesMap;

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::{
    fs::{read_to_string, remove_file},
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

    if output_taxonomies_file.exists() {
        remove_file(output_taxonomies_file.to_owned()).unwrap();
    }

    let taxonomies_database_content =
        read_to_string(taxonomies_database_path).expect("Unable to read file");

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
        write_or_append_to_file(&output_taxonomies_file);

    tax_writer(
        format!("Feature ID\tTaxon\n"),
        tax_file
            .try_clone()
            .expect("Unexpected error detected on write taxonomies database"),
    )?;

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
        .for_each(|line| {
            if let Err(err) = tax_writer(line, tax_file
                .try_clone()
                .expect("Unexpected error detected on write taxonomies database"))
            {
                panic!("Unexpected error detected on write taxonomies database: {err}");
            };
        });

    // ? -----------------------------------------------------------------------
    // ? Validate and parse the blast database
    // ? -----------------------------------------------------------------------

    let er_msg = "Invalid line detected on blastdbcmd response";

    validate_blast_database(blast_database_path)?;

    output_sequences_file.set_extension("fna");

    if output_sequences_file.exists() {
        remove_file(output_sequences_file.to_owned()).unwrap();
    }

    let (fna_writer, fna_file) =
        write_or_append_to_file(&output_sequences_file);

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

                let (accession, taxid, sequence_hash, sequence) = (
                    line.next().expect(er_msg).trim(),
                    line.next().expect(er_msg).trim(),
                    line.next().expect(er_msg).trim(),
                    line.next().expect(er_msg).trim(),
                );

                if let Err(err) = fna_writer(
                    format!(
                        ">{taxid}-{sequence_hash}-{accession}\n{sequence}\n"
                    ),
                    fna_file.try_clone().expect(
                        "Unexpected error detected on write sequences database",
                    ),
                ) {
                    panic!("Unexpected error detected on write sequences database: {err}");
                };

                buf_line.clear();
            }
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(())
}
