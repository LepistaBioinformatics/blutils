use clean_base::utils::errors::{factories::use_case_err, MappedErrors};
use log::debug;
use polars_core::prelude::*;
use polars_io::prelude::*;
use polars_ops::prelude::DataFrameJoinOps;
use rayon::prelude::{ParallelBridge, ParallelIterator};
use std::{
    collections::HashMap,
    env::temp_dir,
    fs::{create_dir, remove_file, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
    sync::Mutex,
};

pub(super) fn build_taxonomy_database(
    names_path: PathBuf,
    nodes_path: PathBuf,
    lineage_path: PathBuf,
    threads: usize,
) -> Result<DataFrame, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Validate arguments
    // ? -----------------------------------------------------------------------

    if !lineage_path.is_file() {
        return use_case_err(format!(
            "Invalid lineages path: {:?}",
            lineage_path
        ))
        .as_error();
    }

    if !nodes_path.is_file() {
        return use_case_err(format!("Invalid nodes path: {:?}", nodes_path))
            .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Load reference data-frames
    // ? -----------------------------------------------------------------------

    debug!("");
    debug!("Loading and validating taxonomic nodes");
    let nodes_df = get_nodes_dataframe(nodes_path, threads)?;

    debug!("");
    debug!("Loading and validating taxonomic names");
    let names_df = match get_names_dataframe(names_path, threads)?.join(
        &nodes_df,
        ["tax_id"],
        ["tax_id"],
        JoinType::Inner,
        None,
    ) {
        Ok(df) => df,
        Err(err) => {
            return use_case_err(format!(
                "Unexpected error detected on join names and nodes: {err}"
            ))
            .as_error();
        }
    };

    debug!("");
    debug!("Loading and validating taxonomic lineages");
    let lineage_names_df = match get_lineage_dataframe(lineage_path, threads)?
        .join(&names_df, ["tax_id"], ["tax_id"], JoinType::Inner, None)
    {
        Ok(df) => df,
        Err(err) => {
            return use_case_err(format!(
                "Unexpected error detected on join names, nodes, with lineages: {err}"
            ))
            .as_error();
        }
    };

    let reduced_df =
        match lineage_names_df.select(["tax_id", "rank", "lineage"]) {
            Ok(df) => df,
            Err(err) => {
                return use_case_err(format!(
                "Unexpected error detected build taxonomies dataframe: {err}"
            ))
                .as_error();
            }
        };

    // ? -----------------------------------------------------------------------
    // ? Fold taxonomies
    // ? -----------------------------------------------------------------------

    debug!("Building fully qualified taxonomies");

    let binding = reduced_df.clone();
    let mut iters = binding
        .columns(["tax_id", "rank", "lineage"])
        .unwrap()
        .iter()
        .map(|s| s.iter())
        .collect::<Vec<_>>();

    println!(
        "unique rank:\n{:?}",
        reduced_df.column("rank").unwrap().unique()
    );

    let binding_unique_lineages =
        binding.column("lineage").unwrap().unique().unwrap();

    let unique_lineages: Vec<&str> = binding_unique_lineages
        .utf8()
        .unwrap()
        .into_iter()
        .filter_map(|i| match i {
            Some(i) => Some(i.trim()),
            None => None,
        })
        .collect();

    for lineage in unique_lineages {
        println!("{:?}", lineage);
    }

    let mut ranked_tax_ids: HashMap<i32, String> = HashMap::new();
    for row in 0..binding.height() {
        ranked_tax_ids.insert(
            iters[0].next().unwrap().to_string().parse::<i32>().unwrap(),
            iters[1].next().unwrap().to_string().trim().to_string(),
        );

        if row > 5 {
            break;
        }
    }

    println!("ranked_tax_ids: {:?}", ranked_tax_ids);

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(reduced_df)
}

/// Loads names dataframe from taxdump
fn get_names_dataframe(
    path: PathBuf,
    threads: usize,
) -> Result<DataFrame, MappedErrors> {
    let column_definitions = vec![
        ("tax_id".to_string(), DataType::Int64),
        ("text_name".to_string(), DataType::Utf8),
        ("unique_name".to_string(), DataType::Utf8),
        ("name_class".to_string(), DataType::Utf8),
    ];

    match load_named_dataframe(path, column_definitions, threads) {
        Ok(df) => {
            match df
                .select(["tax_id".to_string(), "name_class".to_string()])
                .unwrap()
                .filter(
                    &df.column("name_class")
                        .unwrap()
                        .utf8()
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

/// Loads lineage dataframe from taxdump
fn get_lineage_dataframe(
    path: PathBuf,
    threads: usize,
) -> Result<DataFrame, MappedErrors> {
    let column_definitions = vec![
        ("tax_id".to_string(), DataType::Int64),
        ("lineage".to_string(), DataType::Utf8),
    ];

    load_named_dataframe(path, column_definitions, threads)
}

/// Loads nodes dataframe from taxdump
fn get_nodes_dataframe(
    path: PathBuf,
    threads: usize,
) -> Result<DataFrame, MappedErrors> {
    let column_definitions = vec![
        ("tax_id".to_string(), DataType::Int64),
        ("parent_tax_id".to_string(), DataType::Int64),
        ("rank".to_string(), DataType::Utf8),
    ];

    load_named_dataframe(path, column_definitions, threads)
}

/// Loads a dataframe from a file
fn load_named_dataframe(
    path: PathBuf,
    column_definitions: Vec<(String, DataType)>,
    threads: usize,
) -> Result<DataFrame, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Create columns and schema as usable vectors
    // ? -----------------------------------------------------------------------

    debug!("Validating dataframe schema");

    let mut schema = Schema::new();
    let mut columns = Vec::<String>::new();

    for (name, column_type) in &column_definitions {
        schema.with_column(name.to_owned().into(), column_type.to_owned());
        columns.push(name.to_owned());
    }

    // ? -----------------------------------------------------------------------
    // ? Replace default taxdump separator by a simple tabulation
    // ? -----------------------------------------------------------------------

    debug!("Translating taxdump file to a tabular format");

    let tmp_dir = temp_dir().join("blutils");

    if !tmp_dir.exists() {
        match create_dir(tmp_dir.to_owned()) {
            Err(err) => {
                return use_case_err(format!(
                    "Unexpected error detected on create temporary directory: {err}"
                ))
                .as_error()
            }
            Ok(res) => res,
        };
    }

    let temp_file_path = tmp_dir.to_owned().join(path.file_name().unwrap());

    debug!("Temporary content written to {:?}", temp_file_path);

    if temp_file_path.exists() {
        remove_file(temp_file_path.to_owned()).unwrap();
    }

    let reader = match File::open(path) {
        Ok(file) => BufReader::new(file),
        Err(err) => {
            return use_case_err(format!(
                "Unexpected error detected on open temporary file: {err}"
            ))
            .as_error()
        }
    };

    let writer = match File::create(temp_file_path.to_owned()) {
        Err(err) => {
            return use_case_err(format!(
                "Unexpected error detected on read temporary file: {err}"
            ))
            .as_error()
        }
        Ok(file) => Mutex::new(BufWriter::new(file)),
    };

    reader.lines().par_bridge().for_each(|line| {
        let line = line
            .unwrap()
            .to_string()
            .replace("\t|\t", "\t")
            .replace("|\t", "\t")
            .replace("\t|", "\t");

        writeln!(writer.lock().unwrap(), "{}", line).unwrap();
    });

    // ? -----------------------------------------------------------------------
    // ? Load dataframe itself
    // ? -----------------------------------------------------------------------

    debug!("Loading dataframe itself");

    let sequential_columns = columns
        .iter()
        .enumerate()
        .map(|(i, _)| format!("column_{}", i + 1))
        .collect::<Vec<String>>();

    match CsvReader::from_path(temp_file_path) {
        Ok(reader) => {
            let mut df = reader
                .with_delimiter(b'\t')
                .has_header(false)
                .with_columns(Some(sequential_columns))
                .with_n_threads(Some(threads))
                .finish()
                .unwrap();

            for (i, (column, _type)) in schema.iter().enumerate() {
                df.rename(
                    format!("column_{}", i + 1).as_str(),
                    column.to_string().as_str(),
                )
                .unwrap();
            }

            Ok(df)
        }
        Err(err) => {
            return use_case_err(format!(
                "Unexpected error occurred on load table: {err}",
            ))
            .as_error()
        }
    }
}
