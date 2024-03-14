use super::load_dump_file;

use mycelium_base::utils::errors::MappedErrors;
use polars_core::prelude::*;
use std::path::PathBuf;

/// Loads lineage dataframe from taxdump
pub(super) fn load_lineage_dataframe(
    path: PathBuf,
) -> Result<DataFrame, MappedErrors> {
    let column_definitions = vec![
        ("tax_id".to_string(), DataType::Int64),
        ("lineage".to_string(), DataType::String),
    ];

    load_dump_file(path, column_definitions)
}
