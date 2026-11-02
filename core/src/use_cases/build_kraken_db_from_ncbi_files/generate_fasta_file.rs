use crate::use_cases::shared::{
    append, remove_if_exists, validate_blast_database, write_or_append_to_file,
};

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
};
use subprocess::Exec;
use tracing::warn;

pub(super) fn generate_fasta_file(
    blast_database_path: &PathBuf,
    mut output_file: PathBuf,
) -> Result<Vec<(String, usize)>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Validate the blast database
    // ? -----------------------------------------------------------------------

    let er_msg = "Invalid line detected on blastdbcmd response";

    validate_blast_database(blast_database_path)?;

    // ? -----------------------------------------------------------------------
    // ? Initialize the headers vector
    // ? -----------------------------------------------------------------------

    let mut headers = Vec::<(String, usize)>::new();

    // ? -----------------------------------------------------------------------
    // ? Generate the fasta file
    // ? -----------------------------------------------------------------------

    output_file.set_extension("fna");

    if output_file.exists() {
        warn!("Output file already exists. Removing it.");
    }

    // BUG-R43: file-system errors return `Err` instead of panicking.
    remove_if_exists(&output_file)?;

    let (file_writer, file) = write_or_append_to_file(&output_file)?;

    match Exec::cmd("blastdbcmd")
        .arg("-entry")
        .arg("all")
        .arg("-db")
        .arg(blast_database_path)
        .arg("-outfmt")
        .arg("%a  %T  %s")
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

                let invalid_line = || {
                    execution_err(format!(
                        "{er_msg}: {:?}",
                        buf_line.trim_end()
                    ))
                };
                let mut line = buf_line.split("  ");
                let mut field =
                    || line.next().map(str::trim).ok_or_else(invalid_line);

                // BUG-R43: a malformed `blastdbcmd` line returns `Err`.
                let (accession, taxid, sequence) =
                    (field()?, field()?, field()?);
                let parsed_taxid = taxid.parse::<usize>().map_err(|err| {
                    execution_err(format!(
                        "{er_msg}: invalid taxid {taxid:?}: {err}"
                    ))
                })?;

                append(
                    file_writer,
                    &file,
                    format!(
                        ">kraken:taxid|{taxid}|{accession}\n{sequence}\n",
                        sequence = sequence
                            .to_uppercase()
                            .chars()
                            .collect::<Vec<char>>()
                            .chunks(80)
                            .collect::<Vec<&[char]>>()
                            .iter()
                            .map(|x| x.iter().collect::<String>())
                            .collect::<Vec<String>>()
                            .join("\n")
                    ),
                )?;

                headers.push((accession.to_owned(), parsed_taxid));

                buf_line.clear();
            }
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(headers)
}
