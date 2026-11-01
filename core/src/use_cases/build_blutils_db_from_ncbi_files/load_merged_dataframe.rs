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
            let column = |name: &str| {
                match df.column(name) {
                Ok(col) => Ok(col.i64().unwrap().clone()),
                Err(err) => use_case_err(format!(
                    "Unexpected error detected on get merged nodes dataframe: {err}"
                ))
                .as_error(),
            }
            };

            let merged_nodes = column("tax_id")?;
            let new_tax_ids = column("new_tax_id")?;

            // Pair the columns row by row and drop a row as a whole when
            // either side is null (BUG-R15).
            let merged_map = merged_nodes
                .into_iter()
                .zip(&new_tax_ids)
                .filter_map(|(tax_id, new_tax_id)| {
                    Some((tax_id? as u64, new_tax_id? as u64))
                })
                .collect::<HashMap<u64, u64>>();

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::write;

    fn dump(dir: &tempfile::TempDir, content: &str) -> PathBuf {
        let path = dir.path().join("merged.dmp");
        write(&path, content).unwrap();
        path
    }

    #[test]
    fn load_merged_dataframe_ncbi_line_maps_old_to_new_tax_id() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(
            load_merged_dataframe(dump(&dir, "10\t|\t20\t|\n")).unwrap(),
            HashMap::from([(10, 20)])
        );
    }

    #[test]
    fn load_merged_dataframe_multiple_lines_map_each_pair() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(
            load_merged_dataframe(dump(&dir, "10\t|\t20\t|\n11\t|\t21\t|\n"))
                .unwrap(),
            HashMap::from([(10, 20), (11, 21)])
        );
    }

    #[test]
    fn load_merged_dataframe_nonexistent_path_returns_err() {
        let dir = tempfile::tempdir().unwrap();

        let err =
            load_merged_dataframe(dir.path().join("missing.dmp")).unwrap_err();

        assert!(format!("{err}").contains(
            "Unexpected error detected on load merged nodes dataframe"
        ));
    }

    // BUG-R15: a row with a null side is dropped without shifting the others.
    #[test]
    fn load_merged_dataframe_row_with_null_is_dropped_as_a_whole() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(
            load_merged_dataframe(dump(
                &dir,
                "\t|\t20\t|\n10\t|\tabc\t|\n11\t|\t21\t|\n"
            ))
            .unwrap(),
            HashMap::from([(11, 21)])
        );
    }
}
