use crate::{
    domain::dtos::blast_result::ValidTaxonomicRanksEnum,
    use_cases::shared::write_or_append_to_file,
};

use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use polars_core::prelude::*;
use polars_io::prelude::*;
use polars_ops::{
    frame::{JoinArgs, JoinType},
    prelude::DataFrameJoinOps,
};
//use rayon::prelude::{ParallelBridge, ParallelIterator};
use slugify::slugify;
use std::{
    collections::{HashMap, HashSet},
    env::temp_dir,
    fs::{create_dir, remove_file, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
    str::FromStr,
    sync::Mutex,
};
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub(crate) struct RankedTaxidUnit {
    pub name: String,
    pub rank: String,
    pub lineage: String,
}

#[tracing::instrument(name = "Build Taxonomy DB", skip(accessions_map))]
pub(super) fn build_taxonomy_database(
    names_path: PathBuf,
    nodes_path: PathBuf,
    lineage_path: PathBuf,
    del_nodes_path: PathBuf,
    merged_path: PathBuf,
    accessions_map: HashSet<(String, u64)>,
    ignore_taxids: Option<Vec<u64>>,
    replace_rank: Option<HashMap<String, String>>,
    drop_non_linnaean_taxonomies: Option<bool>,
    output_path: PathBuf,
    threads: usize,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Validate arguments
    // ? -----------------------------------------------------------------------

    if !names_path.is_file() {
        return use_case_err(format!("Invalid names path: {:?}", names_path))
            .as_error();
    }

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

    if !del_nodes_path.is_file() {
        return use_case_err(format!(
            "Invalid delnodes path: {:?}",
            del_nodes_path
        ))
        .as_error();
    }

    if !merged_path.is_file() {
        return use_case_err(format!("Invalid merged path: {:?}", merged_path))
            .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Load reference data-frames
    // ? -----------------------------------------------------------------------

    debug!("Loading and validating `DELETED` nodes");
    let del_nodes_vector = get_del_nodes_dataframe(del_nodes_path, threads)?;

    debug!("Loading and validating `MERGED` nodes");
    let merged_map = get_merged_dataframe(merged_path, threads)?;

    debug!("Loading and validating `NAMES`");
    let names_df = get_names_dataframe(names_path, threads)?;

    debug!("Loading and validating `NODES`");

    let nodes_df = get_nodes_dataframe(nodes_path, threads)?;

    debug!("Loading and validating `LINEAGES`");

    let lineages_df = get_lineage_dataframe(lineage_path, threads)?;

    debug!("Joining `NODES` and `LINEAGES`");

    let nodes_and_lineages_df = match nodes_df.join(
        &lineages_df,
        ["tax_id"],
        ["tax_id"],
        JoinArgs {
            how: JoinType::Inner,
            ..Default::default()
        },
    ) {
        Ok(df) => df,
        Err(err) => {
            return use_case_err(format!(
                "Unexpected error detected on join names and nodes: {err}"
            ))
            .as_error();
        }
    };

    debug!("Joining `NODES` and `LINEAGES` with `NAMES`");

    let nodes_and_lineages_with_names_df = match nodes_and_lineages_df.join(
        &names_df,
        ["tax_id"],
        ["tax_id"],
        JoinArgs {
            how: JoinType::Left,
            ..Default::default()
        },
    ) {
        Ok(df) => df,
        Err(err) => {
            return use_case_err(format!(
                "Unexpected error detected on join names, nodes, with lineages: {err}"
            ))
            .as_error();
        }
    };

    debug!("Reducing `NODES` and `LINEAGES` with `NAMES`");

    let reduced_df = match nodes_and_lineages_with_names_df.select([
        "tax_id",
        "rank",
        "text_name",
        "lineage",
    ]) {
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
        .columns(["tax_id", "rank", "text_name", "lineage"])
        .unwrap()
        .iter()
        .map(|s| s.iter())
        .collect::<Vec<_>>();

    let mut ranked_tax_ids: HashMap<u64, RankedTaxidUnit> = HashMap::new();
    for _ in 0..binding.height() {
        let tax_id = iters[0]
            .next()
            .unwrap()
            .to_string()
            .trim()
            .parse::<i32>()
            .unwrap() as u64;

        let rank = iters[1]
            .next()
            .unwrap()
            .to_string()
            .trim()
            .to_string()
            .replace("\"", "")
            .to_lowercase();

        let name = iters[2]
            .next()
            .unwrap()
            .to_string()
            .trim()
            .to_string()
            .replace("\"", "");

        let lineage = iters[3]
            .next()
            .unwrap()
            .to_string()
            .trim()
            .to_string()
            .replace("\"", "");

        ranked_tax_ids.insert(
            tax_id,
            RankedTaxidUnit {
                rank,
                name: match name {
                    name if name.is_empty() || name == "null" => {
                        format!("taxid-{tax_id}")
                    }
                    name => name,
                },
                lineage,
            },
        );
    }

    // ? -----------------------------------------------------------------------
    // ? Build output files
    // ? -----------------------------------------------------------------------

    //
    // Create the textual taxonomies file
    //
    let text_taxonomies_file = match output_path.parent() {
        Some(parent) => parent.join(format!(
            "{}.text.taxonomies.tsv",
            output_path.file_stem().unwrap().to_str().unwrap()
        )),
        None => PathBuf::from("text.taxonomies.tsv"),
    };

    let text_taxonomies_file_binding = text_taxonomies_file.as_path();

    if text_taxonomies_file_binding.exists() {
        remove_file(text_taxonomies_file_binding).unwrap();
    }

    //
    // Create the numeric taxonomies file
    //
    let numeric_taxonomies_file = match output_path.parent() {
        Some(parent) => parent.join(format!(
            "{}.numeric.taxonomies.tsv",
            output_path.file_stem().unwrap().to_str().unwrap()
        )),
        None => PathBuf::from("numeric.taxonomies.tsv"),
    };

    let numeric_taxonomies_file_binding = numeric_taxonomies_file.as_path();

    if numeric_taxonomies_file_binding.exists() {
        remove_file(numeric_taxonomies_file_binding).unwrap();
    }

    //
    // Create a file to include not mapped tax_ids
    //
    let non_mapped_file = match output_path.parent() {
        Some(parent) => parent.join(format!(
            "{}.non-mapped.tsv",
            output_path.file_stem().unwrap().to_str().unwrap()
        )),
        None => PathBuf::from("non-mapped.tsv"),
    };

    let non_mapped_file_file_binding = non_mapped_file.as_path();

    if non_mapped_file_file_binding.exists() {
        remove_file(non_mapped_file_file_binding).unwrap();
    }

    // ? -----------------------------------------------------------------------
    // ? Hydrate lineages and Build the output taxonomies dataframe
    // ? -----------------------------------------------------------------------

    accessions_map.into_iter().for_each(|(accession, tax_id)| {
        let header = format!("{}.{}", accession, tax_id);

        let ranked_tax_id = match ranked_tax_ids.get(&tax_id) {
            Some(res) => res,
            None => {
                //
                // This condition is triggered when a tax_id is not found in the
                // taxdump files and is a deleted node.
                //
                if del_nodes_vector.contains(&tax_id) {
                    match write_or_append_to_file(
                        format!("{}\t{}\n", header, "deleted"),
                        non_mapped_file_file_binding,
                    ) {
                        Ok(_) => (),
                        Err(err) => panic!("{err}")
                    };

                    return;
                }

                //
                // This condition is triggered when a tax_id is not found in the
                // taxdump files and is a merged node.
                //
                if let Some(new_tax_id) = merged_map.get(&tax_id) {
                    match ranked_tax_ids.get(&new_tax_id) {
                        Some(res) => res,
                        None => {

                            match write_or_append_to_file(
                                format!("{}\t{}\n", header, "merged"),
                                non_mapped_file_file_binding,
                            ) {
                                Ok(_) => (),
                                Err(err) => panic!("{err}")
                            };

                            return;
                        }
                    }
                } else {
                    match write_or_append_to_file(
                        format!("{}\t{}\n", header, "unknown"
                    ),
                        non_mapped_file_file_binding,
                    ) {
                        Ok(_) => (),
                        Err(err) => panic!("{err}")
                    };

                    return;
                }
            }
        };

        let lineage = ranked_tax_id.lineage
            .split(" ")
            .flat_map(|lineage_tax_id| {
                if lineage_tax_id.is_empty() || lineage_tax_id == "null" {
                    return None;
                }

                let lineage_tax_id = lineage_tax_id.trim().parse::<u64>().unwrap();

                if let Some(taxids) = ignore_taxids.to_owned() {
                    if taxids.contains(&lineage_tax_id) {
                        return None;
                    }
                }

                let record = match ranked_tax_ids.get(&lineage_tax_id) {
                    Some(res) => res,
                    None => {
                        warn!(
                            "Unmapped tax_id detected {lineage_tax_id} in lineage: {lineage}",
                            lineage = ranked_tax_id.lineage
                        );

                        return None;
                    }
                };

                let valid_rank = match record.rank.parse::<ValidTaxonomicRanksEnum>() {
                    Ok(res) => match res {
                        //
                        // Skip non linnaean taxonomies if the non-linnaean rank
                        // was found and the `drop_non_linnaean_taxonomies` flag
                        // is set to true.
                        //
                        ValidTaxonomicRanksEnum::Other(rank) => {
                            if let Some(true) = drop_non_linnaean_taxonomies {
                                return None;
                            } else {
                                slugify!(rank.clone().as_str(), separator = "-")
                            }
                        }
                        _ => res.to_string(),
                    },
                    Err(_) => {
                        panic!(
                            "Unexpected error detected on parse rank: {}", record.rank
                        )
                    },
                };

                let valid_rank = match replace_rank.to_owned() {
                    Some(replace_rank) => {
                        if let Some(replaced_rank) = replace_rank.get(&valid_rank) {
                            replaced_rank.to_string()
                        } else {
                            valid_rank
                        }
                    }
                    None => valid_rank,
                };

                let ranked_name = format!(
                    "{}__{}",
                    valid_rank,
                    slugify!(record.name.as_str())
                );

                let ranked_taxid = format!(
                    "{}__{}",
                    valid_rank,
                    lineage_tax_id
                );

                Some((ranked_taxid, ranked_name))
            })
            .collect::<Vec<(String, String)>>();

        //
        // Skip non linnaean taxonomies if the non-linnaean rank was found and
        // the `drop_non_linnaean_taxonomies` flag is set to true.
        //
        let slug_rank = match ValidTaxonomicRanksEnum::from_str(&ranked_tax_id.rank) {
            Ok(res) => match res {
                ValidTaxonomicRanksEnum::Other(rank) => {
                    if let Some(true) = drop_non_linnaean_taxonomies {
                        return;
                    } else {
                        slugify!(rank.as_str(), separator = "-")
                    }
                }
                _ => res.to_string(),
            },
            Err(_) => slugify!(ranked_tax_id.rank.as_str(), separator = "-"),
        };

        //
        // Write the names based taxonomies to the output files
        //
        let ranked_names = lineage
            .iter()
            .map(|(_, ranked_name)| ranked_name.to_string())
            .collect::<Vec<String>>()
            .join(";");

        match write_or_append_to_file(format!(
            "{header}\t{ranked_names};{rank}__{name}\n",
            header = header,
            ranked_names = ranked_names,
            rank = slug_rank,
            name = slugify!(ranked_tax_id.name.as_str())
        ), text_taxonomies_file_binding) {
            Ok(_) => (),
            Err(err) => {
                panic!(
                    "Unexpected error detected on write names taxonomy file: {err}"
                );
            }
        };

        //
        // Write the taxi-ds based taxonomies to the output files
        //
        let ranked_taxids = lineage
            .iter()
            .map(|(ranked_taxid,_)| ranked_taxid.to_string())
            .collect::<Vec<String>>()
            .join(";");

        match write_or_append_to_file(format!(
            "{header}\t{ranked_taxids};{rank}__{tax_id}\n",
            header = header,
            ranked_taxids = ranked_taxids,
            rank = slug_rank,
            tax_id = tax_id
        ), numeric_taxonomies_file_binding) {
            Ok(_) => (),
            Err(err) => {
                panic!(
                    "Unexpected error detected on write tax-ids taxonomy file: {err}"
                );
            }
        };
    });

    Ok(())
}

/// Loads names dataframe from taxdump
fn get_names_dataframe(
    path: PathBuf,
    threads: usize,
) -> Result<DataFrame, MappedErrors> {
    let column_definitions = vec![
        ("tax_id".to_string(), DataType::Int64),
        ("text_name".to_string(), DataType::String),
        ("unique_name".to_string(), DataType::String),
        ("name_class".to_string(), DataType::String),
    ];

    match load_named_dataframe(path, column_definitions, threads) {
        Ok(df) => {
            match df
                .select([
                    "tax_id".to_string(),
                    "text_name".to_string(),
                    "name_class".to_string(),
                ])
                .unwrap()
                .filter(
                    &df.column("name_class")
                        .unwrap()
                        .str()
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

/// Loads nodes dataframe from taxdump
fn get_nodes_dataframe(
    path: PathBuf,
    threads: usize,
) -> Result<DataFrame, MappedErrors> {
    let column_definitions = vec![
        ("tax_id".to_string(), DataType::Int64),
        ("parent_tax_id".to_string(), DataType::Int64),
        ("rank".to_string(), DataType::String),
    ];

    load_named_dataframe(path, column_definitions, threads)
}

/// Loads lineage dataframe from taxdump
fn get_lineage_dataframe(
    path: PathBuf,
    threads: usize,
) -> Result<DataFrame, MappedErrors> {
    let column_definitions = vec![
        ("tax_id".to_string(), DataType::Int64),
        ("lineage".to_string(), DataType::String),
    ];

    load_named_dataframe(path, column_definitions, threads)
}

/// Loads nodes dataframe from taxdump
fn get_del_nodes_dataframe(
    path: PathBuf,
    threads: usize,
) -> Result<Vec<u64>, MappedErrors> {
    let column_definitions = vec![("tax_id".to_string(), DataType::Int64)];
    let df = load_named_dataframe(path, column_definitions, threads);

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

fn get_merged_dataframe(
    path: PathBuf,
    threads: usize,
) -> Result<HashMap<u64, u64>, MappedErrors> {
    let column_definitions = vec![
        ("tax_id".to_string(), DataType::Int64),
        ("new_tax_id".to_string(), DataType::Int64),
    ];

    match load_named_dataframe(path, column_definitions, threads) {
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

    reader.lines().for_each(|line| {
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
                .with_separator(b'\t')
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
