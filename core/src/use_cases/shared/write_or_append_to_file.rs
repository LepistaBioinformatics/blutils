use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::{fs::OpenOptions, io::Write, path::Path};
use tracing::error;

pub(crate) fn write_or_append_to_file(
    content: String,
    output_file: &Path,
) -> Result<bool, MappedErrors> {
    match OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(output_file)
        .unwrap()
        .write(content.as_bytes())
    {
        Err(err) => {
            error!("Unexpected error detected: {}", err);
            execution_err(String::from(
                "Unexpected error detected on write file.",
            ))
            .as_error()
        }
        Ok(_) => Ok(true),
    }
}
