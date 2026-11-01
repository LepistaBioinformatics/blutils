use super::load_dump_file;

use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use polars_core::prelude::*;
use std::path::PathBuf;

/// Loads nodes dataframe from taxdump
pub(super) fn load_del_nodes_dataframe(
    path: PathBuf,
) -> Result<Vec<u64>, MappedErrors> {
    let column_definitions = vec![("tax_id".to_string(), DataType::Int64)];
    let df = load_dump_file(path, column_definitions);

    match df {
        Ok(df) => {
            let del_nodes = match df.column("tax_id") {
                Ok(col) => col
                    .i64()
                    .map_err(|err| {
                        use_case_err(format!(
                            "Unexpected error detected on get deleted nodes dataframe: {err}"
                        ))
                    })?
                    .into_iter()
                    .flatten()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::write;

    fn dump(dir: &tempfile::TempDir, content: &str) -> PathBuf {
        let path = dir.path().join("delnodes.dmp");
        write(&path, content).unwrap();
        path
    }

    #[test]
    fn load_del_nodes_dataframe_ncbi_lines_return_tax_ids_in_order() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(
            load_del_nodes_dataframe(dump(&dir, "1\t|\n2\t|\n")).unwrap(),
            vec![1, 2]
        );
    }

    #[test]
    fn load_del_nodes_dataframe_blank_and_non_numeric_lines_are_skipped() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(
            load_del_nodes_dataframe(dump(&dir, "7\t|\n\nabc\t|\n9\t|\n"))
                .unwrap(),
            vec![7, 9]
        );
    }

    #[test]
    fn load_del_nodes_dataframe_nonexistent_path_returns_err() {
        let dir = tempfile::tempdir().unwrap();

        let err = load_del_nodes_dataframe(dir.path().join("missing.dmp"))
            .unwrap_err();

        assert!(format!("{err}").contains(
            "Unexpected error detected on load deleted nodes dataframe"
        ));
    }
}
