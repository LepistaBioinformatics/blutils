use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::{fs::OpenOptions, io::Write, path::Path};

pub(crate) type FileWriter =
    fn(String, std::fs::File) -> Result<(), MappedErrors>;

/// A general purpose function to write or append to a file.
///
/// This function allows to keep the file open and write to it multiple times.
/// BUG-R43: an unopenable file returns `Err` instead of panicking.
pub(crate) fn write_or_append_to_file(
    output_file: &Path,
) -> Result<(FileWriter, std::fs::File), MappedErrors> {
    fn writer(
        content: String,
        mut builder: std::fs::File,
    ) -> Result<(), MappedErrors> {
        builder.write_all(content.as_bytes()).map_err(|err| {
            execution_err(format!("Could not write to file: {err}"))
        })
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_file)
        .map_err(|err| {
            execution_err(format!(
                "Could not open {output_file:?} for writing: {err}"
            ))
        })?;

    Ok((writer, file))
}

/// Removes `path` when it exists, so the caller can write it from scratch.
pub(crate) fn remove_if_exists(path: &Path) -> Result<(), MappedErrors> {
    if path.exists() {
        std::fs::remove_file(path).map_err(|err| {
            execution_err(format!(
                "Could not overwrite the output file {path:?}: {err}"
            ))
        })?;
    }

    Ok(())
}

/// Writes `content` through `writer` on a clone of `file`.
pub(crate) fn append(
    writer: FileWriter,
    file: &std::fs::File,
    content: String,
) -> Result<(), MappedErrors> {
    let handle = file.try_clone().map_err(|err| {
        execution_err(format!("Could not reuse the output file: {err}"))
    })?;

    writer(content, handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::read_to_string;

    #[test]
    fn write_or_append_to_file_two_writes_concatenate() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.txt");

        let (writer, file) = write_or_append_to_file(&path).unwrap();
        writer("first".to_string(), file.try_clone().unwrap()).unwrap();
        writer("second\n".to_string(), file.try_clone().unwrap()).unwrap();

        assert_eq!(read_to_string(&path).unwrap(), "firstsecond\n");
    }

    #[test]
    fn write_or_append_to_file_existing_file_is_appended_not_truncated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.txt");
        std::fs::write(&path, "existing\n").unwrap();

        let (writer, file) = write_or_append_to_file(&path).unwrap();
        writer("new\n".to_string(), file).unwrap();

        assert_eq!(read_to_string(&path).unwrap(), "existing\nnew\n");
    }

    // BUG-R43: used to panic with "Unable to open file".
    #[test]
    fn write_or_append_to_file_nonexistent_parent_returns_err_with_path() {
        let dir = tempfile::tempdir().unwrap();

        let message =
            write_or_append_to_file(&dir.path().join("missing/out.txt"))
                .err()
                .unwrap()
                .to_string();

        assert!(message.contains("Could not open"), "{message}");
        assert!(message.contains("missing/out.txt"), "{message}");
    }
}
