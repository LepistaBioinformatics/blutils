use crate::use_cases::shared::{
    validate_blast_database, write_or_append_to_file,
};

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::{
    fs::remove_file,
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
        remove_file(output_file.to_owned()).unwrap();
    }

    let (file_writer, file) = write_or_append_to_file(&output_file);

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

                let mut line = buf_line.split("  ");

                let (accession, taxid, sequence) = (
                    line.next().expect(er_msg).trim(),
                    line.next().expect(er_msg).trim(),
                    line.next().expect(er_msg).trim(),
                );

                if let Err(err) = file_writer(
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
                    file.try_clone().expect(
                        "Unexpected error detected on write sequences database",
                    ),
                ) {
                    panic!("Unexpected error detected on write sequences database: {err}");
                };

                headers.push((accession.to_owned(), taxid.parse().unwrap()));

                buf_line.clear();
            }
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(headers)
}
