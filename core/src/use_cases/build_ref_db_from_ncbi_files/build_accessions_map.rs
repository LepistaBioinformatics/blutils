use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    path::PathBuf,
};
use subprocess::Exec;
use tracing::warn;

pub(super) fn build_accessions_map(
    blast_database_path: &str,
) -> Result<HashMap<u64, Vec<String>>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Create output files
    // ? -----------------------------------------------------------------------

    let mut taxids_map = HashMap::<u64, Vec<String>>::new();
    let invalid_line = "null";

    // ? -----------------------------------------------------------------------
    // ? Validate blast database
    // ? -----------------------------------------------------------------------

    let mut copy_blast_database_path =
        PathBuf::from(blast_database_path.to_owned());

    copy_blast_database_path.set_extension("nsq");

    if !copy_blast_database_path.exists() {
        return execution_err(format!(
            "Blast database not found: {:?}",
            copy_blast_database_path
        ))
        .as_error();
    }

    let copy_taxdb_path = copy_blast_database_path
        .parent()
        .unwrap_or(&PathBuf::from(blast_database_path))
        .join("taxdb.btd");

    if !copy_taxdb_path.exists() {
        return execution_err(format!(
            "Taxdb not found: {:?}",
            copy_taxdb_path
        ))
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Extract identifiers from blast database
    // ? -----------------------------------------------------------------------

    match Exec::cmd("blastdbcmd")
        .arg("-entry")
        .arg("all")
        .arg("-db")
        .arg(blast_database_path)
        .arg("-outfmt")
        .arg("%a  %T")
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

                let (accession, taxid) = (
                    line.next()
                        .unwrap_or(invalid_line)
                        .split(".")
                        .next()
                        .unwrap_or(invalid_line),
                    line.next().unwrap_or(invalid_line).trim(),
                );

                if accession == invalid_line || taxid == invalid_line {
                    warn!("Invalid line detected on blastdbcmd response");
                    continue;
                }

                let taxid = match taxid.parse::<i64>() {
                    Ok(res) => res,
                    Err(err) => {
                        panic!("Invalid taxid detected on blastdbcmd response for {taxid}: {err}");
                    }
                };

                taxids_map
                    .entry(taxid as u64)
                    .or_insert_with(Vec::new)
                    .push(accession.to_string());

                buf_line.clear();
            }
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(taxids_map)
}
