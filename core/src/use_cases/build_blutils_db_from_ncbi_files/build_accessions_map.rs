use crate::{
    domain::dtos::taxonomies_map::Accession,
    use_cases::shared::validate_blast_database,
};

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    path::PathBuf,
};
use subprocess::Exec;

pub(super) fn build_accessions_map(
    blast_database_path: &str,
) -> Result<HashMap<u64, Vec<Accession>>, MappedErrors> {
    let mut taxids_map = HashMap::<u64, Vec<Accession>>::new();

    validate_blast_database(&PathBuf::from(blast_database_path))?;

    // ? -----------------------------------------------------------------------
    // ? Extract identifiers from blast database
    // ? -----------------------------------------------------------------------

    let er_msg = "Invalid line detected on blastdbcmd response";

    match Exec::cmd("blastdbcmd")
        .arg("-entry")
        .arg("all")
        .arg("-db")
        .arg(blast_database_path)
        .arg("-outfmt")
        .arg("%a  %T  %o")
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

                let (accession, taxid, sequence_oid) = (
                    line.next().expect(er_msg).trim(),
                    line.next().expect(er_msg).trim(),
                    line.next().expect(er_msg).trim(),
                );

                let taxid = match taxid.parse::<i64>() {
                    Ok(res) => res,
                    Err(err) => {
                        panic!("Invalid taxid detected on blastdbcmd response for {taxid}: {err}");
                    }
                };

                taxids_map
                    .entry(taxid as u64)
                    .or_insert_with(Vec::new)
                    .push(Accession {
                        accession: accession.to_string(),
                        oid: sequence_oid.to_string(),
                    });

                buf_line.clear();
            }
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(taxids_map)
}
