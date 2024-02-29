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
        match builder.write(content.as_bytes()) {
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
