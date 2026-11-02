use super::load_dump_file;

use mycelium_base::utils::errors::MappedErrors;
use polars_core::prelude::*;
use std::path::PathBuf;

/// Loads nodes dataframe from taxdump
pub(super) fn load_nodes_dataframe(
    path: PathBuf,
) -> Result<DataFrame, MappedErrors> {
    let column_definitions = vec![
        ("tax_id".to_string(), DataType::Int64),
        ("parent_tax_id".to_string(), DataType::Int64),
        ("rank".to_string(), DataType::String),
    ];

    load_dump_file(path, column_definitions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::write;

    #[test]
    fn load_nodes_dataframe_ncbi_lines_keep_first_three_columns() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nodes.dmp");
        write(
            &path,
            "1\t|\t1\t|\tno rank\t|\t\t|\t8\t|\t0\t|\t1\t|\t0\t|\t0\t|\t0\t|\t0\t|\t0\t|\t\t|\n\
             2\t|\t131567\t|\tsuperkingdom\t|\t\t|\t0\t|\t0\t|\t11\t|\t0\t|\t0\t|\t0\t|\t0\t|\t0\t|\t\t|\n",
        )
        .unwrap();

        let df = load_nodes_dataframe(path).unwrap();

        assert_eq!(
            df.get_column_names(),
            vec!["tax_id", "parent_tax_id", "rank"]
        );
        assert_eq!(df.column("tax_id").unwrap().dtype(), &DataType::Int64);
        assert_eq!(
            df.column("parent_tax_id").unwrap().dtype(),
            &DataType::Int64
        );
        assert_eq!(df.column("rank").unwrap().dtype(), &DataType::String);
        assert_eq!(
            df.column("parent_tax_id")
                .unwrap()
                .i64()
                .unwrap()
                .into_iter()
                .collect::<Vec<_>>(),
            vec![Some(1), Some(131567)]
        );
        assert_eq!(
            df.column("rank")
                .unwrap()
                .str()
                .unwrap()
                .into_iter()
                .collect::<Vec<_>>(),
            vec![Some("no rank"), Some("superkingdom")]
        );
    }

    #[test]
    fn load_nodes_dataframe_nonexistent_path_returns_err() {
        let dir = tempfile::tempdir().unwrap();

        assert!(load_nodes_dataframe(dir.path().join("missing.dmp")).is_err());
    }
}
