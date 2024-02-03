use super::load_dump_file;

use mycelium_base::utils::errors::MappedErrors;
use polars_core::prelude::*;
use std::path::PathBuf;

/// Loads nodes dataframe from taxdump
pub(super) fn load_nodes_dataframe(
    path: PathBuf,
    threads: usize,
) -> Result<DataFrame, MappedErrors> {
    let column_definitions = vec![
        ("tax_id".to_string(), DataType::Int64),
        ("parent_tax_id".to_string(), DataType::Int64),
        ("rank".to_string(), DataType::String),
    ];

    load_dump_file(path, column_definitions, threads)
}
