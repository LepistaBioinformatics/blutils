use mycelium_base::utils::errors::{execution_err, MappedErrors};
use shellexpand::tilde;
use std::path::PathBuf;

pub(crate) fn validate_blast_database(
    blast_database_path: &PathBuf,
) -> Result<(), MappedErrors> {
    let stem = blast_database_path
        .file_stem()
        .unwrap()
        .to_os_string()
        .into_string()
        .unwrap();

    let database_dir = PathBuf::from(
        tilde(&blast_database_path.parent().unwrap().to_str().unwrap())
            .into_owned(),
    );

    // Accept only `{stem}.nsq` or numbered volumes `{stem}.NN.nsq`, so a
    // different database sharing the prefix does not match (BUG-R28).
    let is_database_volume = |file_name: &str| match file_name
        .strip_prefix(stem.as_str())
        .and_then(|rest| rest.strip_suffix(".nsq"))
    {
        Some("") => true,
        Some(volume) => volume.strip_prefix('.').is_some_and(|number| {
            !number.is_empty() && number.chars().all(|c| c.is_ascii_digit())
        }),
        None => false,
    };

    // A bare database name has an empty parent: look in the current directory.
    let search_dir = if database_dir.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        database_dir
    };

    let nsq_candidates = if let Some(path) = std::fs::read_dir(&search_dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(is_database_volume)
        }) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir, File};
    use std::path::Path;

    fn touch(path: &Path) {
        File::create(path).unwrap();
    }

    fn error_message(path: &PathBuf) -> String {
        format!("{}", validate_blast_database(path).unwrap_err())
    }

    #[test]
    fn validate_blast_database_numbered_volume_and_taxdb_returns_ok() {
        let dir = tempfile::tempdir().unwrap();
        touch(&dir.path().join("db.00.nsq"));
        touch(&dir.path().join("taxdb.btd"));

        assert!(validate_blast_database(&dir.path().join("db.fna")).is_ok());
    }

    #[test]
    fn validate_blast_database_unnumbered_volume_returns_ok() {
        let dir = tempfile::tempdir().unwrap();
        touch(&dir.path().join("db.nsq"));
        touch(&dir.path().join("taxdb.btd"));

        assert!(validate_blast_database(&dir.path().join("db.fna")).is_ok());
    }

    // BUG-R28: a different database whose name merely starts with the stem
    // must not validate.
    #[test]
    fn validate_blast_database_other_database_sharing_prefix_returns_not_found_error(
    ) {
        let dir = tempfile::tempdir().unwrap();
        touch(&dir.path().join("dbX.nsq"));
        touch(&dir.path().join("dbX.00.nsq"));
        touch(&dir.path().join("db.x.nsq"));
        touch(&dir.path().join("taxdb.btd"));

        assert!(error_message(&dir.path().join("db.fna"))
            .contains("Blast database not found"));
    }

    // BUG-R28: numbered volumes may have more than two digits.
    #[test]
    fn validate_blast_database_three_digit_volume_returns_ok() {
        let dir = tempfile::tempdir().unwrap();
        touch(&dir.path().join("db.100.nsq"));
        touch(&dir.path().join("taxdb.btd"));

        assert!(validate_blast_database(&dir.path().join("db.fna")).is_ok());
    }

    #[test]
    fn validate_blast_database_missing_nsq_returns_not_found_error() {
        let dir = tempfile::tempdir().unwrap();
        touch(&dir.path().join("db.fna"));
        touch(&dir.path().join("taxdb.btd"));

        assert!(error_message(&dir.path().join("db.fna"))
            .contains("Blast database not found"));
    }

    #[test]
    fn validate_blast_database_missing_taxdb_returns_taxdb_error() {
        let dir = tempfile::tempdir().unwrap();
        touch(&dir.path().join("db.00.nsq"));

        assert!(error_message(&dir.path().join("db.fna"))
            .contains("Taxdb not found"));
    }

    #[test]
    fn validate_blast_database_taxdb_in_parent_dir_returns_taxdb_error() {
        let dir = tempfile::tempdir().unwrap();
        let db_dir = dir.path().join("db");
        create_dir(&db_dir).unwrap();
        touch(&db_dir.join("db.00.nsq"));
        touch(&dir.path().join("taxdb.btd"));

        assert!(
            error_message(&db_dir.join("db.fna")).contains("Taxdb not found")
        );
    }

    /// `~` is expanded through `$HOME`; the tempdir is created under it so the
    /// test environment is never mutated.
    #[test]
    fn validate_blast_database_tilde_path_is_expanded() {
        let home = PathBuf::from(std::env::var("HOME").unwrap());
        let dir = tempfile::Builder::new()
            .prefix(".blutils-test-")
            .tempdir_in(&home)
            .unwrap();
        touch(&dir.path().join("db.00.nsq"));
        touch(&dir.path().join("taxdb.btd"));

        let relative = dir.path().strip_prefix(&home).unwrap();
        let tilde_path = PathBuf::from("~").join(relative).join("db.fna");

        assert!(validate_blast_database(&tilde_path).is_ok());
    }

    #[test]
    #[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
    fn validate_blast_database_root_path_panics() {
        let _ = validate_blast_database(&PathBuf::from("/"));
    }

    #[test]
    #[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
    fn validate_blast_database_parent_dir_path_panics() {
        let _ = validate_blast_database(&PathBuf::from(".."));
    }
}
