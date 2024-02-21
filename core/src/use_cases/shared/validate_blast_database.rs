use mycelium_base::utils::errors::{execution_err, MappedErrors};
use shellexpand::tilde;
use std::path::PathBuf;

pub(crate) fn validate_blast_database(
    blast_database_path: &PathBuf,
) -> Result<(), MappedErrors> {
    let database_name = format!(
        "{path}*.nsq",
        path = blast_database_path
            .file_stem()
            .unwrap()
            .to_os_string()
            .into_string()
            .unwrap()
            .as_str()
    );

    let pattern: PathBuf = [
        &tilde(&blast_database_path.parent().unwrap().to_str().unwrap()),
        database_name.as_str(),
    ]
    .iter()
    .collect();

    let nsq_candidates = if let Some(path) =
        glob::glob(&pattern.to_string_lossy())
            .expect("Failed to read glob pattern")
            .find_map(Result::ok)
    {
        path
    } else {
        return execution_err(format!(
            "Blast database not found: {:?}",
            blast_database_path
        ))
        .as_error();
    };

    if let false = nsq_candidates
        .parent()
        .expect(
            format!(
                "Could not determine database directory from: {:?}",
                nsq_candidates
            )
            .as_str(),
        )
        .join("taxdb.btd")
        .exists()
    {
        return execution_err(format!(
            "Taxdb not found: {:?}",
            nsq_candidates.parent().unwrap()
        ))
        .as_error();
    };

    Ok(())
}
