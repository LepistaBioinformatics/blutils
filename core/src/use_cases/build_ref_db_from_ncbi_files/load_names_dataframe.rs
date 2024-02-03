use super::load_dump_file;

use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use polars_core::prelude::*;
use std::path::PathBuf;

/// Loads names dataframe from taxdump
pub(super) fn load_names_dataframe(
    path: PathBuf,
    threads: usize,
) -> Result<DataFrame, MappedErrors> {
    let column_definitions = vec![
        ("tax_id".to_string(), DataType::Int64),
        ("text_name".to_string(), DataType::String),
        ("unique_name".to_string(), DataType::String),
        ("name_class".to_string(), DataType::String),
    ];

    match load_dump_file(path, column_definitions, threads) {
        Ok(df) => {
            match df
                .select([
                    "tax_id".to_string(),
                    "text_name".to_string(),
                    "name_class".to_string(),
                ])
                .unwrap()
                .filter(
                    &df.column("name_class")
                        .unwrap()
                        .str()
                        .unwrap()
                        .equal("scientific name"),
                ) {
                Ok(df) => Ok(df),
                Err(err) => {
                    return use_case_err(
                        format!("Unexpected error detected on filter names dataframe: {err}")
                    )
                    .as_error();
                }
            }
        }
        Err(err) => {
            return use_case_err(format!(
                "Unexpected error detected on load names dataframe: {err}"
            ))
            .as_error();
        }
    }
}
