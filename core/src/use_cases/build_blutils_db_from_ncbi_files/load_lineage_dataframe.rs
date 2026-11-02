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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::write;

    #[test]
    fn load_lineage_dataframe_ncbi_lines_have_tax_id_and_lineage_columns() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("taxidlineage.dmp");
        write(&path, "1\t|\t\t|\n1423\t|\t1 131567 2 1239 \t|\n").unwrap();

        let df = load_lineage_dataframe(path).unwrap();

        assert_eq!(df.get_column_names(), vec!["tax_id", "lineage"]);
        assert_eq!(df.column("tax_id").unwrap().dtype(), &DataType::Int64);
        assert_eq!(df.column("lineage").unwrap().dtype(), &DataType::String);
        assert_eq!(
            df.column("tax_id")
                .unwrap()
                .i64()
                .unwrap()
                .into_iter()
                .collect::<Vec<_>>(),
            vec![Some(1), Some(1423)]
        );
        assert_eq!(
            df.column("lineage")
                .unwrap()
                .str()
                .unwrap()
                .into_iter()
                .collect::<Vec<_>>(),
            vec![Some(""), Some("1 131567 2 1239")]
        );
    }

    #[test]
    fn load_lineage_dataframe_nonexistent_path_returns_err() {
        let dir = tempfile::tempdir().unwrap();

        assert!(load_lineage_dataframe(dir.path().join("missing.dmp")).is_err());
    }
}
