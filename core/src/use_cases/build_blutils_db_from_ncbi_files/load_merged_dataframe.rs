use super::load_dump_file;

use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use polars_core::prelude::*;
use std::{collections::HashMap, path::PathBuf};

pub(super) fn load_merged_dataframe(
    path: PathBuf,
) -> Result<HashMap<u64, u64>, MappedErrors> {
    let column_definitions = vec![
        ("tax_id".to_string(), DataType::Int64),
        ("new_tax_id".to_string(), DataType::Int64),
    ];

    match load_dump_file(path, column_definitions) {
        Ok(df) => {
            let merged_nodes = match df.column("tax_id") {
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
                        "Unexpected error detected on get merged nodes dataframe: {err}"
                    ))
                    .as_error()
                }
            };

            let new_tax_ids = match df.column("new_tax_id") {
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
                        "Unexpected error detected on get merged nodes dataframe: {err}"
                    ))
                    .as_error()
                }
            };

            let mut merged_map = HashMap::new();
            for (tax_id, new_tax_id) in
                merged_nodes.iter().zip(new_tax_ids.iter())
            {
                merged_map.insert(*tax_id as u64, *new_tax_id as u64);
            }

            Ok(merged_map)
        }
        Err(err) => {
            return use_case_err(format!(
            "Unexpected error detected on load merged nodes dataframe: {err}"
        ))
            .as_error()
        }
    }
}
