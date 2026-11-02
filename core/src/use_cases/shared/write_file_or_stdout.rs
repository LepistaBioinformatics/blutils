use super::write_or_append_to_file::{append, FileWriter};
use mycelium_base::utils::errors::MappedErrors;

pub(crate) fn write_or_stdout(
    content: String,
    writer: FileWriter,
    file: &std::fs::File,
    stdout: bool,
) -> Result<(), MappedErrors> {
    if stdout {
        println!("{}", content);
        return Ok(());
    }

    // Terminate each row like `println!` does (BUG-R10). A write error
    // returns `Err` instead of panicking (BUG-R43).
    append(writer, file, content + "\n")
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
        let (writer, file) = write_or_append_to_file(&path).unwrap();

        write_or_stdout("a\tb".to_string(), writer, &file, false).unwrap();
        write_or_stdout("c\td".to_string(), writer, &file, false).unwrap();

        assert_eq!(read_to_string(&path).unwrap(), "a\tb\nc\td\n");
    }

    // BUG-R43: used to panic with a misleading "sequences database" message.
    #[test]
    fn write_or_stdout_file_branch_writer_error_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let file = std::fs::File::create(dir.path().join("out.tsv")).unwrap();

        let result = write_or_stdout(
            "a".to_string(),
            |_, _| execution_err("boom".to_string()).as_error(),
            &file,
            false,
        );

        assert!(result.err().unwrap().to_string().contains("boom"));
    }
}
