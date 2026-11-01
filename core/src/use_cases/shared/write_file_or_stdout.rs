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
        // Terminate each row like `println!` does (BUG-R10).
        if let Err(err) = writer(
            content + "\n",
            file.try_clone()
                .expect("Unexpected error detected on write tabular output"),
        ) {
            panic!(
                "Unexpected error detected on write sequences database: {err}"
            );
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::shared::write_or_append_to_file;
    use mycelium_base::utils::errors::execution_err;
    use std::fs::read_to_string;

    // BUG-R10: the file branch terminates each row with `\n`, as the stdout
    // branch does through `println!`.
    #[test]
    fn write_or_stdout_file_branch_terminates_each_row_with_newline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.tsv");
        let (writer, file) = write_or_append_to_file(&path);

        write_or_stdout(
            "a\tb".to_string(),
            writer,
            file.try_clone().unwrap(),
            false,
        );
        write_or_stdout("c\td".to_string(), writer, file, false);

        assert_eq!(read_to_string(&path).unwrap(), "a\tb\nc\td\n");
    }

    #[test]
    #[should_panic(
        expected = "Unexpected error detected on write sequences database"
    )]
    fn write_or_stdout_file_branch_writer_error_panics() {
        let dir = tempfile::tempdir().unwrap();
        let file = std::fs::File::create(dir.path().join("out.tsv")).unwrap();

        write_or_stdout(
            "a".to_string(),
            |_, _| execution_err("boom".to_string()).as_error(),
            file,
            false,
        );
    }
}
