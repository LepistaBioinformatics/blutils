use super::{filter_rank_by_identity, run_parallel_blast::ParallelBlastOutput};
use crate::domain::dtos::{
    blast_builder::{BlastBuilder, Taxon},
    blast_result::{
        BlastQueryConsensusResult, BlastQueryNoConsensusResult,
        BlastQueryResult, BlastResultRow, ConsensusResult, TaxonomyElement,
        TaxonomyFieldEnum, ValidTaxonomicRanksEnum,
    },
};

use clean_base::utils::errors::{execution_err, MappedErrors};
use log::{error, warn};
use polars::prelude::{CsvReader, DataFrame, DataType, Schema};
use polars_io::prelude::*;
use polars_lazy::prelude::*;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize, clap::ValueEnum)]
pub enum ConsensusStrategy {
    Cautious,
    Relaxed,
}

/// BUild consensus identities from BlastN output.
///
/// Join the `blast` output with reference `taxonomies` file and calculate
/// consensus taxonomies based on the `subjects` frequencies and concordance.
pub(crate) fn build_consensus_identities(
    blast_output: ParallelBlastOutput,
    taxonomies_file: &Path,
    config: BlastBuilder,
    strategy: ConsensusStrategy,
) -> Result<Vec<ConsensusResult>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Load blast output as lazy
    // ? ----------------------------------------------------------------------

    let blast_output_df = match get_results_dataframe(&blast_output.output_file)
    {
        Err(err) => {
            error!("Unexpected error detected on read `blast_output`: {}", err);

            return Err(execution_err(
                String::from(
                    "Unexpected error detected on read `blast_output`",
                ),
                None,
                None,
            ));
        }
        Ok(res) => res,
    };

    // ? ----------------------------------------------------------------------
    // ? Load taxonomies as lazy
    // ? ----------------------------------------------------------------------

    let taxonomies_df = match get_taxonomies_dataframe(taxonomies_file) {
        Err(err) => {
            return Err(execution_err(
                format!(
                    "Unexpected error detected on read `taxonomies_file`: {err}",
                ),
                None,
                None,
            ));
        }
        Ok(res) => res,
    };

    // ? ----------------------------------------------------------------------
    // ? Merge files as lazy
    // ? ----------------------------------------------------------------------

    let joined_df = blast_output_df.lazy().left_join(
        taxonomies_df.lazy(),
        col("subject"),
        col("subject"),
    );

    // ? ----------------------------------------------------------------------
    // ? Build consensus vector
    // ? ----------------------------------------------------------------------

    let mut query_results = match fold_results_by_query(joined_df) {
        Err(err) => return Err(err),
        Ok(res) => res,
    };

    let mut remaining_query_results = Vec::<BlastQueryResult>::new();

    let comparing_query_results = query_results
        .iter()
        .map(|result| result.query.to_owned())
        .collect::<Vec<String>>();

    blast_output.headers.into_iter().for_each(|header| {
        if !comparing_query_results.contains(&header) {
            remaining_query_results.push(BlastQueryResult {
                query: header,
                results: None,
            });
        };
    });

    query_results.append(&mut remaining_query_results);

    query_results
        .into_par_iter()
        .map(|result| {
            if result.results.to_owned().is_none() {
                return Ok(ConsensusResult::NoConsensusFound(
                    BlastQueryNoConsensusResult {
                        query: result.query,
                    },
                ));
            }

            match find_single_query_consensus(
                result.query,
                result.results.unwrap(),
                config.to_owned(),
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

fn find_single_query_consensus(
    query: String,
    result: Vec<BlastResultRow>,
    config: BlastBuilder,
    strategy: ConsensusStrategy,
) -> Result<ConsensusResult, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Group results by bit-score
    // ? -----------------------------------------------------------------------

    let grouped_results = result.to_owned().into_iter().fold(
        HashMap::<i64, Vec<BlastResultRow>>::new(),
        |mut init, result| {
            init.entry(result.bit_score)
                .or_insert_with(Vec::new)
                .push(result);

            init
        },
    );

    // ? -----------------------------------------------------------------------
    // ? Evaluate results by bit-score
    // ? -----------------------------------------------------------------------

    let mut sorted_keys = grouped_results.keys().cloned().collect::<Vec<i64>>();
    sorted_keys.sort_by(|a, b| b.cmp(a));

    let no_consensus = BlastQueryNoConsensusResult {
        query: query.to_owned(),
    };

    for score in sorted_keys.to_owned().into_iter() {
        let score_results = result
            .to_owned()
            .into_iter()
            .filter(|i| i.bit_score == score)
            .map(|mut i| i.parse_taxonomy())
            .collect::<Vec<BlastResultRow>>();
        //
        // Early return case no results found.
        //
        if score_results.len() == 0 {
            return Ok(ConsensusResult::NoConsensusFound(no_consensus));
        }
        //
        // Fetch the lower taxonomic rank case only one record returned.
        //
        if score_results.len() == 1 {
            for rank in ValidTaxonomicRanksEnum::ordered_iter(None) {
                match score_results[0].taxonomy.to_owned() {
                    TaxonomyFieldEnum::Parsed(taxonomies) => {
                        match taxonomies.into_iter().find(|i| &i.rank == rank) {
                            None => {
                                return Ok(ConsensusResult::NoConsensusFound(
                                    no_consensus,
                                ))
                            }
                            Some(mut res) => {
                                let rank = match filter_rank_by_identity(
                                    config.to_owned().taxon.to_owned(),
                                    score_results[0].perc_identity,
                                    res.rank,
                                ) {
                                    Err(err) => panic!("{err}"),
                                    Ok(res) => res,
                                };

                                res.rank = rank;

                                return Ok(ConsensusResult::ConsensusFound(
                                    BlastQueryConsensusResult {
                                        query,
                                        taxon: Some(res),
                                    },
                                ));
                            }
                        }
                    }
                    _ => panic!("Unable to parse taxonomy."),
                };
            }

            return Ok(ConsensusResult::NoConsensusFound(no_consensus));
        }
        //
        // Fetch the lower taxonomic rank case more than one record returned.
        //
        if score_results.len() > 1 {
            match find_multi_taxa_consensus(
                score_results,
                config.to_owned().taxon,
                no_consensus.clone(),
                strategy.to_owned(),
            ) {
                Err(err) => panic!("{err}"),
                Ok(res) => return Ok(res),
            };
        }
    }

    // Execute the default option
    //
    // If consensus identity not found in the previous steps, assumes by default
    // a no consensus option.
    Ok(ConsensusResult::NoConsensusFound(no_consensus))
}

/// Find the consensus among Blast results with multiple output.
///
/// In some cases blast results returns a list if records with the same percent
/// identity and bit-score. In this cases this logic could be applied to solve
/// the problem.
fn find_multi_taxa_consensus(
    records: Vec<BlastResultRow>,
    taxon: Taxon,
    no_consensus_option: BlastQueryNoConsensusResult,
    strategy: ConsensusStrategy,
) -> Result<ConsensusResult, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Collect the reference taxonomy vector
    //
    // The taxonomies vector contain elements of the reference taxonomy given
    // the selected strategy. The `Cautious` strategy selects the shortest
    // taxonomic vector as a reference. Otherwise (`Relaxed` strategy), the
    // longest taxonomic vector is selected.
    //
    // ? -----------------------------------------------------------------------

    let mut sorted_records = records.to_owned();

    sorted_records.sort_by(|a, b| {
        let a_taxonomy = force_parsed_taxonomy(a.taxonomy.to_owned());
        let b_taxonomy = force_parsed_taxonomy(b.taxonomy.to_owned());

        a_taxonomy.len().cmp(&b_taxonomy.len())
    });

    let reference_taxonomy = match strategy {
        ConsensusStrategy::Cautious => {
            sorted_records.first().unwrap().to_owned()
        }
        ConsensusStrategy::Relaxed => sorted_records.last().unwrap().to_owned(),
    };

    // ? -----------------------------------------------------------------------
    // ? Build the bi-dimensional
    //
    // Each position of the vector contain a vector of the `TaxonomyElement`
    // type.
    //
    // ? -----------------------------------------------------------------------

    let taxonomies = sorted_records
        .into_iter()
        .map(|i| force_parsed_taxonomy(i.taxonomy))
        .collect::<Vec<Vec<TaxonomyElement>>>();

    let lowest_taxonomy_of_higher_rank =
        get_rank_lowest_statistics(taxonomies.first().unwrap().to_owned());

    // ? -----------------------------------------------------------------------
    // ? Initialize the final response based on high taxonomic rank
    // ? -----------------------------------------------------------------------

    let mut final_taxon = BlastQueryConsensusResult {
        query: no_consensus_option.query.to_owned(),
        taxon: Some(lowest_taxonomy_of_higher_rank),
    };

    // ? -----------------------------------------------------------------------
    // ? Try to update the final response
    // ? -----------------------------------------------------------------------

    let reference_taxonomy_elements =
        force_parsed_taxonomy(reference_taxonomy.taxonomy.to_owned());

    for (index, ref_taxonomy) in reference_taxonomy_elements.iter().enumerate()
    {
        let mut level_taxonomies = Vec::<(ValidTaxonomicRanksEnum, i64)>::new();

        for taxonomy in taxonomies.iter() {
            if index < taxonomy.len() {
                if !level_taxonomies.contains(&(
                    taxonomy[index].rank.to_owned(),
                    taxonomy[index].taxid,
                )) {
                    level_taxonomies.push((
                        taxonomy[index].rank.to_owned(),
                        taxonomy[index].taxid,
                    ));
                }
            }
        }

        if level_taxonomies.len() > 1 {
            final_taxon = build_taxon(
                no_consensus_option.query.to_owned(),
                taxon.to_owned(),
                reference_taxonomy_elements[index - 1].to_owned(),
            );

            break;
        }

        final_taxon = build_taxon(
            no_consensus_option.query.to_owned(),
            taxon.to_owned(),
            ref_taxonomy.to_owned(),
        );
    }

    Ok(ConsensusResult::ConsensusFound(final_taxon))
}

fn build_taxon(
    query: String,
    taxon: Taxon,
    mut element: TaxonomyElement,
) -> BlastQueryConsensusResult {
    let rank = match filter_rank_by_identity(
        taxon.to_owned(),
        element.perc_identity,
        element.rank,
    ) {
        Err(err) => panic!("{err}"),
        Ok(res) => res,
    };

    element.rank = rank;

    BlastQueryConsensusResult {
        query: query,
        taxon: Some(element),
    }
}

/// Get lowest Blast statistics of a single rank vector
///
/// Collect the lowest statistics guarantees that the worst case will be
/// selected, avoiding over-interpretation of the results.
fn get_rank_lowest_statistics(
    mut rank_taxonomies: Vec<TaxonomyElement>,
) -> TaxonomyElement {
    rank_taxonomies
        .sort_by(|a, b| a.perc_identity.partial_cmp(&b.perc_identity).unwrap());

    rank_taxonomies.first().unwrap().to_owned()
}

fn force_parsed_taxonomy(taxonomy: TaxonomyFieldEnum) -> Vec<TaxonomyElement> {
    match taxonomy {
        TaxonomyFieldEnum::Literal(_) => {
            panic!("Invalid format taxonomic field.")
        }
        TaxonomyFieldEnum::Parsed(res) => res,
    }
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
        let mut subject: String = String::new();
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
                1 => subject = value.to_owned().to_string().replace("\"", ""),
                2 => perc_identity = value.try_extract().unwrap(),
                3 => align_length = value.try_extract().unwrap(),
                4 => mismatches = value.try_extract().unwrap(),
                5 => gap_openings = value.try_extract().unwrap(),
                6 => q_start = value.try_extract().unwrap(),
                7 => q_end = value.try_extract().unwrap(),
                8 => s_start = value.try_extract().unwrap(),
                9 => s_end = value.try_extract().unwrap(),
                10 => e_value = value.try_extract().unwrap(),
                11 => bit_score = value.try_extract().unwrap(),
                12 => taxonomy = value.to_owned().to_string().replace("\"", ""),
                _ => warn!("Unmapped value: {:?}", value),
            };

            counter = counter + 1;
        }

        mapped_results.entry(query).or_insert_with(Vec::new).push(
            BlastResultRow {
                subject,
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
                taxonomy: TaxonomyFieldEnum::Literal(taxonomy),
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
        ("query".to_string(), DataType::Utf8),
        ("subject".to_string(), DataType::Utf8),
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

fn get_taxonomies_dataframe(path: &Path) -> Result<DataFrame, MappedErrors> {
    let column_definitions = vec![
        ("subject".to_string(), DataType::Utf8),
        ("taxonomy".to_string(), DataType::Utf8),
    ];

    load_named_dataframe(path, column_definitions, vec![])
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
        schema.with_column(name.to_owned(), column_type.to_owned())
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
            return Err(execution_err(
                String::from("Unexpected error occurred on load table."),
                None,
                None,
            ));
        }
        Ok(res) => Ok(res
            .with_delimiter(b'\t')
            .has_header(false)
            .with_schema(&schema)
            .with_columns(Some(columns_names))
            .finish()
            .unwrap()),
    }
}
