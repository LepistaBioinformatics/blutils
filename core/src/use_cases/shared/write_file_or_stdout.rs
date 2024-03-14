use mycelium_base::utils::errors::MappedErrors;

pub(crate) fn write_or_stdout(
    content: String,
    writer: fn(String, std::fs::File) -> Result<(), MappedErrors>,
    file: std::fs::File,
    stdout: bool,
) {
    if stdout {
        println!("{}", content);
    } else {
        if let Err(err) = writer(
            content,
            file.try_clone()
                .expect("Unexpected error detected on write tabular output"),
        ) {
            panic!(
                "Unexpected error detected on write sequences database: {err}"
            );
        };
    }
}
