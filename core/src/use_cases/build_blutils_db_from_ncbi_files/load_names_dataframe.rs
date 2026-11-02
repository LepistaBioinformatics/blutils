use super::load_dump_file;

use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use polars_core::prelude::*;
use std::path::PathBuf;

/// Loads names dataframe from taxdump
pub(super) fn load_names_dataframe(
    path: PathBuf,
) -> Result<DataFrame, MappedErrors> {
    let column_definitions = vec![
        ("tax_id".to_string(), DataType::Int64),
        ("text_name".to_string(), DataType::String),
        ("unique_name".to_string(), DataType::String),
        ("name_class".to_string(), DataType::String),
    ];

    match load_dump_file(path, column_definitions) {
        Ok(df) => {
            let filter_err = |err: PolarsError| {
                use_case_err(format!(
                    "Unexpected error detected on filter names dataframe: {err}"
                ))
            };

            let is_scientific_name = df
                .column("name_class")
                .and_then(|column| {
                    column.str().map(|names| names.equal("scientific name"))
                })
                .map_err(filter_err)?;

            df.select([
                "tax_id".to_string(),
                "text_name".to_string(),
                "name_class".to_string(),
            ])
            .and_then(|names| names.filter(&is_scientific_name))
            .map_err(filter_err)
        }
        Err(err) => {
            return use_case_err(format!(
                "Unexpected error detected on load names dataframe: {err}"
            ))
            .as_error();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::write;

    fn dump(dir: &tempfile::TempDir, content: &str) -> PathBuf {
        let path = dir.path().join("names.dmp");
        write(&path, content).unwrap();
        path
    }

    const NAMES: &str = "\
1\t|\tall\t|\t\t|\tsynonym\t|
1\t|\troot\t|\t\t|\tscientific name\t|
2\t|\tBacteria\t|\tBacteria <bacteria>\t|\tscientific name\t|
2\t|\teubacteria\t|\t\t|\tgenbank common name\t|
1423\t|\tBacillus subtilis\t|\t\t|\tscientific name\t|
";

    #[test]
    fn load_names_dataframe_keeps_only_scientific_names() {
        let dir = tempfile::tempdir().unwrap();

        let df = load_names_dataframe(dump(&dir, NAMES)).unwrap();

        assert_eq!(
            df.column("tax_id")
                .unwrap()
                .i64()
                .unwrap()
                .into_iter()
                .collect::<Vec<_>>(),
            vec![Some(1), Some(2), Some(1423)]
        );
        assert_eq!(
            df.column("text_name")
                .unwrap()
                .str()
                .unwrap()
                .into_iter()
                .collect::<Vec<_>>(),
            vec![Some("root"), Some("Bacteria"), Some("Bacillus subtilis")]
        );
    }

    #[test]
    fn load_names_dataframe_drops_unique_name_column() {
        let dir = tempfile::tempdir().unwrap();

        let df = load_names_dataframe(dump(&dir, NAMES)).unwrap();

        assert_eq!(
            df.get_column_names(),
            vec!["tax_id", "text_name", "name_class"]
        );
        assert_eq!(df.column("tax_id").unwrap().dtype(), &DataType::Int64);
        assert_eq!(df.column("text_name").unwrap().dtype(), &DataType::String);
    }

    #[test]
    fn load_names_dataframe_nonexistent_path_returns_err() {
        let dir = tempfile::tempdir().unwrap();

        let err =
            load_names_dataframe(dir.path().join("missing.dmp")).unwrap_err();

        assert!(format!("{err}")
            .contains("Unexpected error detected on load names dataframe"));
    }
}
