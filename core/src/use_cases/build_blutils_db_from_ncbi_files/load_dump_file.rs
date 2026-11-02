use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use polars_core::prelude::*;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};
use tracing::{debug, warn};

/// Number of offending raw values quoted in the unparsed-values warning.
const UNPARSED_SAMPLE_SIZE: usize = 3;

/// Loads a dataframe from a file
pub(super) fn load_dump_file(
    path: PathBuf,
    column_definitions: Vec<(String, DataType)>,
) -> Result<DataFrame, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Create columns and schema as usable vectors
    // ? -----------------------------------------------------------------------

    debug!("Build dataframe schema");

    let mut schema = Schema::new();
    let mut columns = Vec::<String>::new();

    for (name, column_type) in &column_definitions {
        schema.with_column(name.to_owned().into(), column_type.to_owned());
        columns.push(name.to_owned());
    }

    // ? -----------------------------------------------------------------------
    // ? Build columns map from file
    // ? -----------------------------------------------------------------------

    debug!("Build columns map from file");

    let mut column_maps = HashMap::<String, Vec<_>>::new();
    match File::open(path.to_owned()) {
        Ok(file) => {
            for (line_index, line) in BufReader::new(file)
                .lines()
                .flat_map(Result::ok)
                .enumerate()
            {
                let values = line
                    .split('|')
                    .map(|value| value.trim().replace("\t", ""))
                    .collect::<Vec<_>>();

                // BUG-R43: a line with fewer fields than expected returns
                // `Err` instead of panicking.
                if values.len() < columns.len() {
                    return use_case_err(format!(
                        "Invalid line {} in {path:?}: expected at least {} `|`-separated fields, found {}",
                        line_index + 1,
                        columns.len(),
                        values.len()
                    ))
                    .as_error();
                }

                for (column, value) in columns.iter().zip(values) {
                    column_maps
                        .entry(column.to_string())
                        .or_insert_with(Vec::new)
                        .push(value);
                }
            }
        }
        Err(err) => {
            return use_case_err(format!(
                "Unexpected error detected on open temporary file: {err}"
            ))
            .as_error()
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Build output dataframe
    // ? -----------------------------------------------------------------------

    debug!("Build output dataframe");

    let mut df = match DataFrame::new(
        column_definitions
            .par_iter()
            .map(|(name, _type)| {
                // An empty dump file has no rows for any column (BUG-R43).
                let series = column_maps
                    .get(name)
                    .map(Vec::as_slice)
                    .unwrap_or_default()
                    .par_iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>();
                Series::new(name, series)
            })
            .collect::<Vec<_>>(),
    ) {
        Ok(df) => df,
        Err(err) => {
            return use_case_err(format!(
                "Unexpected error detected on create dataframe: {err}"
            ))
            .as_error()
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Update dataframe schema
    // ? -----------------------------------------------------------------------

    debug!("Update dataframe schema");

    for (column, _type) in schema.iter() {
        let raw = match df.column(column) {
            Ok(raw) => raw.clone(),
            Err(err) => {
                return use_case_err(format!(
                    "Unexpected error detected on apply column: {err}"
                ))
                .as_error()
            }
        };

        // The cast is non-strict: values that cannot be parsed become null and
        // the affected rows may be dropped downstream, so the user must be
        // warned (BUG-R34).
        let casted = match raw.cast(_type) {
            Ok(casted) => casted,
            Err(err) => {
                return use_case_err(format!(
                    "Unexpected error detected on cast column: {err}"
                ))
                .as_error()
            }
        };

        warn_unparsed_values(&path, column, _type, &raw, &casted);

        if let Err(err) = df.replace(column, casted) {
            return use_case_err(format!(
                "Unexpected error detected on apply column: {err}"
            ))
            .as_error();
        };
    }

    Ok(df)
}

/// Warns when the cast turned non-null raw values into nulls.
fn warn_unparsed_values(
    path: &Path,
    column: &str,
    data_type: &DataType,
    raw: &Series,
    casted: &Series,
) {
    let unparsed = raw.is_not_null() & casted.is_null();
    let count = unparsed.sum().unwrap_or(0);

    if count == 0 {
        return;
    }

    let samples = raw
        .filter(&unparsed)
        .ok()
        .and_then(|filtered| {
            filtered.str().ok().map(|values| {
                values
                    .into_iter()
                    .flatten()
                    .take(UNPARSED_SAMPLE_SIZE)
                    .map(|value| format!("{value:?}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
        })
        .unwrap_or_default();

    warn!(
        "{count} row(s) of `{path}` have a value in column `{column}` that \
        could not be parsed as {data_type}; these values become null and the \
        affected rows may be discarded. First unparsed values: [{samples}]",
        path = path.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::write,
        sync::{Arc, Mutex},
    };

    fn dump(dir: &tempfile::TempDir, content: &str) -> PathBuf {
        let path = dir.path().join("file.dmp");
        write(&path, content).unwrap();
        path
    }

    fn two_columns() -> Vec<(String, DataType)> {
        vec![
            ("tax_id".to_string(), DataType::Int64),
            ("name".to_string(), DataType::String),
        ]
    }

    #[test]
    fn load_dump_file_ncbi_lines_split_on_pipe_and_trim_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dump(
            &dir,
            "2\t|\tBacteria\t|\textra\t|\n1423\t|\t  Bacillus subtilis \t|\n",
        );

        let df = load_dump_file(path, two_columns()).unwrap();

        assert_eq!(df.get_column_names(), vec!["tax_id", "name"]);
        assert_eq!(
            df.column("tax_id")
                .unwrap()
                .i64()
                .unwrap()
                .into_iter()
                .collect::<Vec<_>>(),
            vec![Some(2), Some(1423)]
        );
        assert_eq!(
            df.column("name")
                .unwrap()
                .str()
                .unwrap()
                .into_iter()
                .collect::<Vec<_>>(),
            vec![Some("Bacteria"), Some("Bacillus subtilis")]
        );
    }

    #[test]
    fn load_dump_file_columns_are_cast_to_declared_dtypes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dump(&dir, "2\t|\tBacteria\t|\n");

        let df = load_dump_file(path, two_columns()).unwrap();

        assert_eq!(df.column("tax_id").unwrap().dtype(), &DataType::Int64);
        assert_eq!(df.column("name").unwrap().dtype(), &DataType::String);
    }

    /// Runs `f` under a subscriber that records every event as plain text.
    fn capture_logs<T>(f: impl FnOnce() -> T) -> (T, String) {
        let buffer = Arc::new(Mutex::new(Vec::<u8>::new()));
        let writer = buffer.clone();

        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_ansi(false)
            .with_writer(move || BufferWriter(writer.clone()))
            .finish();

        let result = tracing::subscriber::with_default(subscriber, f);
        let logs = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();

        (result, logs)
    }

    struct BufferWriter(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for BufferWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    // BUG-R34: unparsed numeric values used to become null silently.
    #[test]
    fn load_dump_file_non_numeric_integer_value_warns_with_file_column_and_count(
    ) {
        let dir = tempfile::tempdir().unwrap();
        let path = dump(&dir, "abc\t|\tx\t|\n\t|\ty\t|\n3\t|\tz\t|\n");

        let (df, logs) =
            capture_logs(|| load_dump_file(path.clone(), two_columns()));
        let df = df.unwrap();

        assert_eq!(
            df.column("tax_id")
                .unwrap()
                .i64()
                .unwrap()
                .into_iter()
                .collect::<Vec<_>>(),
            vec![None, None, Some(3)]
        );

        let warnings = logs
            .lines()
            .filter(|line| line.contains("WARN"))
            .collect::<Vec<_>>();

        assert_eq!(warnings.len(), 1, "{logs}");
        assert!(warnings[0].contains("2 row(s)"), "{logs}");
        assert!(warnings[0].contains(&path.display().to_string()), "{logs}");
        assert!(warnings[0].contains("column `tax_id`"), "{logs}");
        assert!(warnings[0].contains(r#"["abc", ""]"#), "{logs}");
    }

    #[test]
    fn load_dump_file_clean_file_emits_no_warning() {
        let dir = tempfile::tempdir().unwrap();
        let path = dump(&dir, "2\t|\tBacteria\t|\n1423\t|\tB. subtilis\t|\n");

        let (df, logs) = capture_logs(|| load_dump_file(path, two_columns()));

        assert_eq!(df.unwrap().height(), 2);
        assert!(!logs.contains("WARN"), "{logs}");
    }

    #[test]
    fn load_dump_file_nonexistent_path_returns_err() {
        let dir = tempfile::tempdir().unwrap();

        let err = load_dump_file(dir.path().join("missing.dmp"), two_columns())
            .unwrap_err();

        assert!(format!("{err}").contains("Unexpected error detected on open"));
    }

    // BUG-R43: used to panic on `Option::unwrap()`.
    #[test]
    fn load_dump_file_line_with_fewer_fields_than_columns_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let path = dump(&dir, "2\t|\tBacteria\t|\n3\n");

        let message =
            load_dump_file(path, two_columns()).unwrap_err().to_string();

        assert!(message.contains("Invalid line 2"), "{message}");
        assert!(message.contains("file.dmp"), "{message}");
    }

    // BUG-R43: used to panic on `Option::unwrap()`.
    #[test]
    fn load_dump_file_empty_file_returns_empty_dataframe() {
        let dir = tempfile::tempdir().unwrap();
        let path = dump(&dir, "");

        let df = load_dump_file(path, two_columns()).unwrap();

        assert_eq!(df.height(), 0);
        assert_eq!(df.get_column_names(), vec!["tax_id", "name"]);
    }
}
