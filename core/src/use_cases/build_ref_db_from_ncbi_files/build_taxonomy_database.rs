use super::{
    load_del_nodes_dataframe, load_lineage_dataframe, load_merged_dataframe,
    load_names_dataframe, load_nodes_dataframe,
};
use crate::{
    domain::dtos::{
        linnaean_ranks::LinnaeanRank,
        taxonomies_map::{TaxonomiesMap, TaxonomyMapUnit},
    },
    use_cases::shared::write_or_append_to_file,
};

use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use polars_ops::{
    frame::{JoinArgs, JoinType},
    prelude::DataFrameJoinOps,
};
use slugify::slugify;
use std::{
    collections::HashMap,
    fs::{remove_file, File},
    io::Write,
    path::PathBuf,
    str::FromStr,
};
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub(crate) struct RankedTaxidUnit {
    pub name: String,
    pub rank: String,
    pub lineage: String,
}

#[tracing::instrument(
    name = "Build Taxonomy DB",
    skip(
        names_path,
        nodes_path,
        lineage_path,
        del_nodes_path,
        merged_path,
        accessions_map,
        ignore_taxids,
        replace_rank,
        drop_non_linnaean_taxonomies,
    )
)]
pub(super) fn build_taxonomy_database(
    names_path: PathBuf,
    nodes_path: PathBuf,
    lineage_path: PathBuf,
    del_nodes_path: PathBuf,
    merged_path: PathBuf,
    accessions_map: HashMap<u64, Vec<String>>,
    ignore_taxids: Option<Vec<u64>>,
    replace_rank: Option<HashMap<String, String>>,
    drop_non_linnaean_taxonomies: Option<bool>,
    database: String,
    output_file_path: PathBuf,
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
    let del_nodes_vector = load_del_nodes_dataframe(del_nodes_path)?;

    debug!("Loading and validating `MERGED` nodes");
    let merged_map = load_merged_dataframe(merged_path)?;

    debug!("Loading and validating `NAMES`");
    let names_df = load_names_dataframe(names_path)?;

    debug!("Loading and validating `NODES`");

    let nodes_df = load_nodes_dataframe(nodes_path)?;

    debug!("Loading and validating `LINEAGES`");

    let lineages_df = load_lineage_dataframe(lineage_path)?;

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

    let mut output_path = output_file_path;
    output_path.set_extension("json");

    //
    // Create the main output file path
    //
    let output_database_file = match output_path.parent() {
        Some(parent) => parent.join(format!(
            "{}.blutils.json",
            output_path.file_stem().unwrap().to_str().unwrap()
        )),
        None => PathBuf::from("blutils.json"),
    };

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

    let mut taxonomies = Vec::<TaxonomyMapUnit>::new();

    accessions_map.into_iter().for_each(|(tax_id, accessions)| {
        let header = format!("{tax_id}");

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

                let valid_rank = match replace_rank.to_owned() {
                    Some(replace_rank) => {
                        if let Some(replaced_rank) = replace_rank.get(&record.rank) {
                            replaced_rank.to_string()
                        } else {
                            record.rank.to_string()
                        }
                    }
                    None => record.rank.to_string(),
                };

                let valid_rank = match valid_rank.parse::<LinnaeanRank>() {
                    Ok(res) => match res {
                        //
                        // Skip non linnaean taxonomies if the non-linnaean rank
                        // was found and the `drop_non_linnaean_taxonomies` flag
                        // is set to true.
                        //
                        LinnaeanRank::Other(rank) => {
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

                let ranked_name = format!(
                    "{}__{}",
                    valid_rank,
                    slugify!(record.name.as_str()).replace("__", "_")
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
        let slug_rank = match LinnaeanRank::from_str(&ranked_tax_id.rank) {
            Ok(res) => match res {
                LinnaeanRank::Other(rank) => {
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
        // Write the taxi-ds based taxonomies to the output files
        //
        let mut ranked_taxids = lineage
            .iter()
            .map(|(ranked_taxid,_)| ranked_taxid.to_string())
            .collect::<Vec<String>>()
            .join(";");

        ranked_taxids = format!("{};{}__{}", ranked_taxids, slug_rank, tax_id);

        //
        // Write the names based taxonomies to the output files
        //
        let mut ranked_names = lineage
            .iter()
            .map(|(_, ranked_name)| ranked_name.to_string())
            .collect::<Vec<String>>()
            .join(";");

        ranked_names = format!(
            "{};{}__{}", 
            ranked_names, 
            slug_rank, 
            slugify!(ranked_tax_id.name.as_str()).replace("__", "_")
        );

        taxonomies.push(TaxonomyMapUnit {
            taxid: tax_id,
            rank: slug_rank,
            numeric_lineage: ranked_taxids,
            text_lineage: ranked_names,
            accessions: accessions.to_owned(),
        });
    });

    let mut file = match File::create(output_database_file) {
        Err(err) => panic!("{err}"),
        Ok(res) => res,
    };

    match file.write_all(
        serde_json::to_string_pretty(&TaxonomiesMap {
            blutils_version: env!("CARGO_PKG_VERSION").to_string(),
            ignore_taxids,
            replace_rank,
            drop_non_linnaean_taxonomies,
            source_database: database,
            taxonomies,
        })
        .unwrap()
        .as_bytes(),
    ) {
        Err(err) => panic!("{err}"),
        Ok(_) => (),
    };

    Ok(())
}
