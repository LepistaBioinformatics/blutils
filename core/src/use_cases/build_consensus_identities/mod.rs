mod build_blast_consensus_identity;
mod find_multi_taxa_consensus;
mod find_single_query_consensus;
mod force_parsed_taxonomy;

use build_blast_consensus_identity::*;
use find_multi_taxa_consensus::*;
use find_single_query_consensus::*;
use force_parsed_taxonomy::*;
use polars_core::{
    export::num::ToPrimitive, prelude::NamedFrom, series::Series,
};

use crate::domain::dtos::{
    blast_builder::Taxon,
    blast_result::{BlastQueryResult, BlastResultRow},
    consensus_result::{ConsensusResult, QueryWithoutConsensus},
    consensus_strategy::ConsensusStrategy,
    parallel_blast_output::ParallelBlastOutput,
    taxonomies_map::TaxonomiesMap,
    taxonomy_bean::Taxonomy,
};

use mycelium_base::utils::errors::{execution_err, use_case_err, MappedErrors};
use polars::prelude::{CsvReader, DataFrame, DataType, Schema};
use polars_io::SerReader;
use polars_lazy::prelude::*;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::{collections::HashMap, fs::read_to_string, path::Path, sync::Arc};
use tracing::{error, info, warn};

/// BUild consensus identities from BlastN output.
///
/// Join the `blast` output with reference `taxonomies` file and calculate
/// consensus taxonomies based on the `subjects` frequencies and concordance.
#[tracing::instrument(
    name = "Build consensus identities from Blast output",
    skip(blast_output, taxonomies_file, taxon, strategy, use_taxid)
)]
pub fn build_consensus_identities(
    blast_output: ParallelBlastOutput,
    taxonomies_file: &Path,
    taxon: Taxon,
    strategy: ConsensusStrategy,
    use_taxid: Option<bool>,
) -> Result<Vec<ConsensusResult>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Load blast output as lazy
    // ? -----------------------------------------------------------------------

    info!("Loading Blast results");

    let blast_output_df = get_results_dataframe(&blast_output.output_file)?;

    info!("Blast results loaded");

    // ? -----------------------------------------------------------------------
    // ? Load taxonomies as lazy
    // ? -----------------------------------------------------------------------

    info!("Loading Blutils taxonomies");

    let taxonomies_df = get_taxonomies_dataframe(taxonomies_file, use_taxid)?;

    info!("Blutils taxonomies loaded");

    // ? -----------------------------------------------------------------------
    // ? Merge files as lazy
    // ? -----------------------------------------------------------------------

    let joined_df = blast_output_df.lazy().left_join(
        taxonomies_df.lazy(),
        col("subject_taxid"),
        col("taxid"),
    );

    // ? -----------------------------------------------------------------------
    // ? Build consensus vector
    // ? -----------------------------------------------------------------------

    let mut query_results = fold_results_by_query(joined_df)?;

    let mut remaining_query_results = Vec::<BlastQueryResult>::new();

    let comparing_query_results = query_results
        .iter()
        .map(|result| result.query.to_owned())
        .collect::<Vec<String>>();

    if let Some(headers) = blast_output.headers {
        headers.into_iter().for_each(|header| {
            if !comparing_query_results.contains(&header) {
                remaining_query_results.push(BlastQueryResult {
                    query: header,
                    results: None,
                });
            };
        });
    }

    query_results.append(&mut remaining_query_results);

    query_results
        .into_par_iter()
        .map(|result| {
            if result.results.to_owned().is_none() {
                return Ok(ConsensusResult::NoConsensusFound(
                    QueryWithoutConsensus {
                        query: result.query,
                    },
                ));
            }

            match find_single_query_consensus(
                result.query,
                result.results.unwrap(),
                taxon.to_owned(),
                strategy.to_owned(),
            ) {
                Err(err) => {
                    panic!("Unexpected error on parse blast results: {err}")
                }
                Ok(res) => Ok(res),
            }
        })
        .collect()
}

/// Group results by query
///
/// Each query results should be grouped into a `BlastQueryResult` struct.
fn fold_results_by_query(
    joined_df: LazyFrame,
) -> Result<Vec<BlastQueryResult>, MappedErrors> {
    let mut binding = joined_df.collect().unwrap();
    let joined_df_chunked = binding.as_single_chunk_par();

    let mut iters = joined_df_chunked
        .iter()
        .map(|s| s.iter())
        .collect::<Vec<_>>();

    let mut mapped_results = HashMap::<String, Vec<BlastResultRow>>::new();

    for _ in 0..joined_df_chunked.height() {
        let mut counter = 0;

        let mut query: String = String::new();
        let mut subject_accession: String = String::new();
        let mut subject_taxid: i64 = 0;
        let mut perc_identity: f64 = 0.0;
        let mut align_length: i64 = 0;
        let mut mismatches: i64 = 0;
        let mut gap_openings: i64 = 0;
        let mut q_start: i64 = 0;
        let mut q_end: i64 = 0;
        let mut s_start: i64 = 0;
        let mut s_end: i64 = 0;
        let mut e_value: f64 = 0.0;
        let mut bit_score: i64 = 0;
        let mut taxonomy: String = String::new();

        for iter in &mut iters {
            let value = iter.next().expect("Not enough rows to iterate.");

            match counter {
                0 => query = value.to_owned().to_string().replace("\"", ""),
                1 => {
                    subject_accession =
                        value.to_owned().to_string().replace("\"", "")
                }
                2 => subject_taxid = value.try_extract().unwrap(),
                3 => perc_identity = value.try_extract().unwrap(),
                4 => align_length = value.try_extract().unwrap(),
                5 => mismatches = value.try_extract().unwrap(),
                6 => gap_openings = value.try_extract().unwrap(),
                7 => q_start = value.try_extract().unwrap(),
                8 => q_end = value.try_extract().unwrap(),
                9 => s_start = value.try_extract().unwrap(),
                10 => s_end = value.try_extract().unwrap(),
                11 => e_value = value.try_extract().unwrap(),
                12 => bit_score = value.try_extract().unwrap(),
                13 => taxonomy = value.to_owned().to_string().replace("\"", ""),
                _ => warn!("Unmapped value: {:?}", value),
            };

            counter = counter + 1;
        }

        mapped_results.entry(query).or_insert_with(Vec::new).push(
            BlastResultRow {
                subject_accession,
                subject_taxid,
                perc_identity,
                align_length,
                mismatches,
                gap_openings,
                q_start,
                q_end,
                s_start,
                s_end,
                e_value,
                bit_score,
                taxonomy: Taxonomy::Literal(taxonomy),
            },
        );
    }

    Ok(mapped_results
        .into_iter()
        .map(|(k, v)| BlastQueryResult {
            query: k,
            results: match v.len() {
                0 => None,
                _ => Some(v),
            },
        })
        .collect::<Vec<BlastQueryResult>>())
}

/// Load BlastN output dataframe.
///
/// The results dataframe is a default tabular option of the Blast results.
fn get_results_dataframe(path: &Path) -> Result<DataFrame, MappedErrors> {
    let column_definitions = vec![
        ("query".to_string(), DataType::String),
        ("subject_accession".to_string(), DataType::String),
        ("subject_taxid".to_string(), DataType::Int64),
        ("perc_identity".to_string(), DataType::Float64),
        ("align_length".to_string(), DataType::Int64),
        ("mismatches".to_string(), DataType::Int64),
        ("gap_openings".to_string(), DataType::Int64),
        ("q_start".to_string(), DataType::Int64),
        ("q_end".to_string(), DataType::Int64),
        ("s_start".to_string(), DataType::Int64),
        ("s_end".to_string(), DataType::Int64),
        ("e_value".to_string(), DataType::Float64),
        ("bit_score".to_string(), DataType::Float64),
    ];

    load_named_dataframe(path, column_definitions, vec![])
}

fn get_taxonomies_dataframe(
    path: &Path,
    use_taxid: Option<bool>,
) -> Result<DataFrame, MappedErrors> {
    let rdr = read_to_string(path).expect("Unable to read file");

    let taxonomy_map = match serde_json::from_str::<TaxonomiesMap>(&rdr) {
        Err(err) => {
            error!("Unexpected error detected on read `taxonomies`: {}", err);
            return execution_err(String::from(
                "Unexpected error occurred on load table.",
            ))
            .as_error();
        }
        Ok(res) => res,
    };

    let column_definitions = vec![
        ("taxid".to_string(), DataType::Int64),
        ("taxonomy".to_string(), DataType::String),
    ];

    let mut df = match DataFrame::new(vec![
        Series::new(
            "taxid",
            taxonomy_map
                .taxonomies
                .iter()
                .map(|item| item.taxid.to_f64().unwrap())
                .collect::<Vec<_>>(),
        ),
        Series::new(
            "taxonomy",
            taxonomy_map
                .taxonomies
                .iter()
                .map(|v| {
                    if let Some(true) = use_taxid {
                        v.numeric_lineage.to_owned()
                    } else {
                        v.text_lineage.to_owned()
                    }
                })
                .collect::<Vec<_>>(),
        ),
    ]) {
        Ok(df) => df,
        Err(err) => {
            return use_case_err(format!(
                "Unexpected error detected on create dataframe: {err}"
            ))
            .as_error()
        }
    };

    let mut schema = Schema::new();

    for (name, column_type) in &column_definitions {
        schema.with_column(name.to_owned().into(), column_type.to_owned());
    }

    for (column, _type) in schema.iter() {
        match df.apply(column, |s| match s.cast(_type) {
            Ok(casted) => casted,
            Err(err) => {
                panic!("Unexpected error detected on cast column: {err}")
            }
        }) {
            Ok(res) => res,
            Err(err) => {
                return use_case_err(format!(
                    "Unexpected error detected on apply column: {err}"
                ))
                .as_error()
            }
        };
    }

    Ok(df)
}

fn load_named_dataframe(
    path: &Path,
    column_definitions: Vec<(String, DataType)>,
    exclude_list: Vec<String>,
) -> Result<DataFrame, MappedErrors> {
    // initialize dataframe schema
    let mut schema = Schema::new();

    // Map definitions to schema
    for (name, column_type) in &column_definitions {
        schema.with_column(name.to_owned().into(), column_type.to_owned());
    }

    // Collect column names
    let mut columns_names: Vec<String> = [].to_vec();

    for (name, _) in &column_definitions {
        // Check if the current column exists inside the `exclude_list` vector.
        // Case `true`, ignore the current column.
        if exclude_list.contains(name) {
            continue;
        }

        // Otherwise, include it into the desired columns vector.
        columns_names.push(name.to_owned());
    }

    // Load dataframe
    match CsvReader::from_path(path) {
        Err(err) => {
            error!("Unexpected error detected on read `blast_output`: {}", err);
            return execution_err(String::from(
                "Unexpected error occurred on load table.",
            ))
            .as_error();
        }
        Ok(res) => Ok(res
            .with_separator(b'\t')
            .has_header(false)
            .with_schema(Some(Arc::new(schema)))
            .with_columns(Some(columns_names))
            .finish()
            .unwrap()),
    }
}
