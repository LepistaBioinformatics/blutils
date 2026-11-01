use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::{fs::OpenOptions, io::Write, path::Path};
use tracing::error;

/// A general purpose function to write or append to a file.
///
/// This function allows to keep the file open and write to it multiple times.
pub(crate) fn write_or_append_to_file(
    output_file: &Path,
) -> (
    fn(String, std::fs::File) -> Result<(), MappedErrors>,
    std::fs::File,
) {
    fn builder(output_file: &Path) -> std::fs::File {
        OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(output_file)
            .expect("Unable to open file")
    }

    fn writer(
        content: String,
        mut builder: std::fs::File,
    ) -> Result<(), MappedErrors> {
        match builder.write_all(content.as_bytes()) {
            Err(err) => {
                error!("Unexpected error detected: {}", err);
                execution_err(String::from(
                    "Unexpected error detected on write file.",
                ))
                .as_error()
            }
            Ok(_) => Ok(()),
        }
    }

    (writer, builder(output_file))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::read_to_string;

    #[test]
    fn write_or_append_to_file_two_writes_concatenate() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.txt");

        let (writer, file) = write_or_append_to_file(&path);
        writer("first".to_string(), file.try_clone().unwrap()).unwrap();
        writer("second\n".to_string(), file.try_clone().unwrap()).unwrap();

        assert_eq!(read_to_string(&path).unwrap(), "firstsecond\n");
    }

    #[test]
    fn write_or_append_to_file_existing_file_is_appended_not_truncated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.txt");
        std::fs::write(&path, "existing\n").unwrap();

        let (writer, file) = write_or_append_to_file(&path);
        writer("new\n".to_string(), file).unwrap();

        assert_eq!(read_to_string(&path).unwrap(), "existing\nnew\n");
    }

    #[test]
    #[should_panic(expected = "Unable to open file")]
    fn write_or_append_to_file_nonexistent_parent_panics() {
        let dir = tempfile::tempdir().unwrap();

        let _ = write_or_append_to_file(&dir.path().join("missing/out.txt"));
    }
}
