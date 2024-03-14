use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use polars_core::prelude::*;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};
use tracing::debug;

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
            BufReader::new(file)
                .lines()
                .flat_map(Result::ok)
                .for_each(|line| {
                    for (i, column) in columns.iter().enumerate() {
                        column_maps
                            .entry(column.to_string())
                            .or_insert_with(Vec::new)
                            .push(
                                line.split('|')
                                    .map(|value| value.trim().replace("\t", ""))
                                    .nth(i)
                                    .unwrap()
                                    .to_string(),
                            );
                    }
                })
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
                let series = column_maps
                    .get(name)
                    .unwrap()
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
        match df.apply(column, |s| match s.cast(_type) {
            Ok(casted) => casted,
            Err(err) => {
                panic!("Unexpected error detected on cast column: {err}")
            }
        }) {
            Ok(res) => res,
            Err(err) => {
                return use_case_err(format!(
                    "Unexpected error detected on apply column: {err}"
                ))
                .as_error()
            }
        };
    }

    Ok(df)
}
