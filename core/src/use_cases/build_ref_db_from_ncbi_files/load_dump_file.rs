use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use polars_core::prelude::*;
use polars_io::prelude::*;
use std::{
    env::temp_dir,
    fs::{create_dir, remove_file, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
    sync::Mutex,
};
use tracing::debug;

/// Loads a dataframe from a file
pub(super) fn load_dump_file(
    path: PathBuf,
    column_definitions: Vec<(String, DataType)>,
    threads: usize,
) -> Result<DataFrame, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Create columns and schema as usable vectors
    // ? -----------------------------------------------------------------------

    debug!("Validating dataframe schema");

    let mut schema = Schema::new();
    let mut columns = Vec::<String>::new();

    for (name, column_type) in &column_definitions {
        schema.with_column(name.to_owned().into(), column_type.to_owned());
        columns.push(name.to_owned());
    }

    // ? -----------------------------------------------------------------------
    // ? Replace default taxdump separator by a simple tabulation
    // ? -----------------------------------------------------------------------

    debug!("Translating taxdump file to a tabular format");

    let tmp_dir = temp_dir().join("blutils");

    if !tmp_dir.exists() {
        match create_dir(tmp_dir.to_owned()) {
            Err(err) => {
                return use_case_err(format!(
                    "Unexpected error detected on create temporary directory: {err}"
                ))
                .as_error()
            }
            Ok(res) => res,
        };
    }

    let temp_file_path = tmp_dir.to_owned().join(path.file_name().unwrap());

    debug!("Temporary content written to {:?}", temp_file_path);

    if temp_file_path.exists() {
        remove_file(temp_file_path.to_owned()).unwrap();
    }

    let reader = match File::open(path) {
        Ok(file) => BufReader::new(file),
        Err(err) => {
            return use_case_err(format!(
                "Unexpected error detected on open temporary file: {err}"
            ))
            .as_error()
        }
    };

    let writer = match File::create(temp_file_path.to_owned()) {
        Err(err) => {
            return use_case_err(format!(
                "Unexpected error detected on read temporary file: {err}"
            ))
            .as_error()
        }
        Ok(file) => Mutex::new(BufWriter::new(file)),
    };

    reader.lines().for_each(|line| {
        let line = line
            .unwrap()
            .to_string()
            .replace("\t|\t", "\t")
            .replace("|\t", "\t")
            .replace("\t|", "\t");

        writeln!(writer.lock().unwrap(), "{}", line).unwrap();
    });

    // ? -----------------------------------------------------------------------
    // ? Load dataframe itself
    // ? -----------------------------------------------------------------------

    debug!("Loading dataframe itself");

    let sequential_columns = columns
        .iter()
        .enumerate()
        .map(|(i, _)| format!("column_{}", i + 1))
        .collect::<Vec<String>>();

    match CsvReader::from_path(temp_file_path) {
        Ok(reader) => {
            let mut df = reader
                .with_separator(b'\t')
                .has_header(false)
                .with_columns(Some(sequential_columns))
                .with_n_threads(Some(threads))
                .finish()
                .unwrap();

            for (i, (column, _type)) in schema.iter().enumerate() {
                df.rename(
                    format!("column_{}", i + 1).as_str(),
                    column.to_string().as_str(),
                )
                .unwrap();
            }

            Ok(df)
        }
        Err(err) => {
            return use_case_err(format!(
                "Unexpected error occurred on load table: {err}",
            ))
            .as_error()
        }
    }
}
