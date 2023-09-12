use std::{
    collections::HashSet,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use clean_base::utils::errors::{factories::execution_err, MappedErrors};
use log::warn;
use subprocess::{Exec, Redirection};

pub(super) fn build_fasta_database(
    blast_database_path: &str,
) -> Result<(PathBuf, HashSet<u64>), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Extract sequences from blast database
    // ? -----------------------------------------------------------------------

    let blast_response = match Exec::cmd("blastdbcmd")
        .arg("-entry")
        .arg("all")
        .arg("-db")
        .arg(blast_database_path)
        .arg("-outfmt")
        .arg("%a  %T  %s")
        .stdout(Redirection::Pipe)
        .stderr(Redirection::Pipe)
        .capture()
    {
        Err(err) => {
            return execution_err(format!(
                "Unexpected error detected on execute blastdbcmd: {err}"
            ))
            .as_error()
        }
        Ok(res) => res,
    };

    if !blast_response.success() {
        return execution_err(blast_response.stderr_str()).as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Build fasta file from previous blast database sequences
    // ? -----------------------------------------------------------------------

    let binding_output_path = format!("{blast_database_path}.fasta");
    let output_path = Path::new(&binding_output_path);

    let mut file = match File::create(output_path) {
        Err(err) => {
            return execution_err(format!(
            "Unexpected error detected on create reference fasta file: {err}"
        ))
            .as_error()
        }
        Ok(res) => res,
    };

    let invalid_line = "null";
    let mut tax_ids = HashSet::<u64>::new();

    for line in blast_response.stdout_str().lines() {
        let mut line = line.split("  ");
        let accession = line
            .next()
            .unwrap_or(invalid_line)
            .split(".")
            .next()
            .unwrap_or(invalid_line);
        let taxid = line.next().unwrap_or(invalid_line);
        let sequence = line.next().unwrap_or(invalid_line);

        if accession == invalid_line ||
            taxid == invalid_line ||
            sequence == invalid_line
        {
            warn!("Invalid line detected on blastdbcmd response");
            continue;
        }

        tax_ids.insert(taxid.parse::<u64>().unwrap_or(0));

        let _ = file.write_all(
            format!(">{accession}.{taxid}\n{sequence}\n").as_bytes(),
        );
    }

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok((output_path.to_path_buf(), tax_ids))
}
