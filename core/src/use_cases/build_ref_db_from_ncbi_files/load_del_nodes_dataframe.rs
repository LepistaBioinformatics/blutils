use super::load_dump_file;

use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use polars_core::prelude::*;
use std::path::PathBuf;

/// Loads nodes dataframe from taxdump
pub(super) fn load_del_nodes_dataframe(
    path: PathBuf,
    threads: usize,
) -> Result<Vec<u64>, MappedErrors> {
    let column_definitions = vec![("tax_id".to_string(), DataType::Int64)];
    let df = load_dump_file(path, column_definitions, threads);

    match df {
        Ok(df) => {
            let del_nodes = match df.column("tax_id") {
                Ok(col) => col
                    .i64()
                    .unwrap()
                    .into_iter()
                    .filter_map(|tax_id| {
                        if tax_id.is_none() {
                            return None;
                        }

                        Some(tax_id.unwrap())
                    })
                    .collect::<Vec<i64>>(),
                Err(err) => {
                    return use_case_err(format!(
                        "Unexpected error detected on get deleted nodes dataframe: {err}"
                    ))
                    .as_error()
                }
            };

            Ok(del_nodes
                .into_iter()
                .map(|tax_id| tax_id as u64)
                .collect::<Vec<u64>>())
        }
        Err(err) => {
            return use_case_err(format!(
            "Unexpected error detected on load deleted nodes dataframe: {err}"
        ))
            .as_error()
        }
    }
}
