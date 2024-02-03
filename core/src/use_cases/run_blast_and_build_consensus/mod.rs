mod build_blast_consensus_identity;
mod build_consensus_identities;
mod filter_rank_by_identity;
mod find_multi_taxa_consensus;
mod find_single_query_consensus;
mod force_parsed_taxonomy;
mod get_rank_lowest_statistics;
mod get_taxonomy_from_position;
mod run_parallel_blast;

use build_blast_consensus_identity::*;
use build_consensus_identities::*;
use filter_rank_by_identity::*;
use find_multi_taxa_consensus::*;
use find_single_query_consensus::*;
use force_parsed_taxonomy::*;
use get_rank_lowest_statistics::*;
use get_taxonomy_from_position::*;
use run_parallel_blast::*;

use crate::domain::{
    dtos::{
        blast_builder::BlastBuilder,
        consensus_result::{ConsensusResult, QueryWithConsensusResult},
        consensus_strategy::ConsensusStrategy,
    },
    entities::execute_blastn::ExecuteBlastn,
};

use mycelium_base::utils::errors::MappedErrors;
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
use tracing::info;

/// Run parallel blast and build taxonomies consensus
#[tracing::instrument(
    name = "Run Blast with Consensus",
    skip(blast_execution_repo)
)]
pub fn run_blast_and_build_consensus(
    input_sequences: &str,
    input_taxonomies: &str,
    out_dir: &str,
    blast_config: BlastBuilder,
    blast_execution_repo: &dyn ExecuteBlastn,
    overwrite: &bool,
    threads: usize,
    strategy: ConsensusStrategy,
) -> Result<bool, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Execute parallel blast
    // ? -----------------------------------------------------------------------

    let output = run_parallel_blast(
        input_sequences,
        out_dir,
        blast_config.to_owned(),
        blast_execution_repo,
        overwrite,
        threads,
    )?;

    // ? -----------------------------------------------------------------------
    // ? Build consensus
    // ? -----------------------------------------------------------------------

    let blast_output = build_consensus_identities(
        output,
        Path::new(input_taxonomies),
        blast_config,
        strategy,
    )?;

    write_json_output(
        blast_output.to_owned(),
        Path::new(out_dir).to_path_buf(),
    );

    Ok(true)
}

fn write_json_output(results: Vec<ConsensusResult>, out_dir: PathBuf) {
    let output_file = out_dir.join("blutils.consensus.json");
    info!("");
    info!("Blutils output file:");
    info!("\t{:?}", output_file);
    info!("");

    let mut file = match File::create(output_file) {
        Err(err) => panic!("{err}"),
        Ok(res) => res,
    };

    let consensus_type_results = results.iter().fold(
        Vec::<QueryWithConsensusResult>::new(),
        |mut init, record| {
            match record {
                ConsensusResult::NoConsensusFound(res) => {
                    init.push(QueryWithConsensusResult {
                        query: res.query.to_owned(),
                        taxon: None,
                    });
                }
                ConsensusResult::ConsensusFound(res) => {
                    init.push(QueryWithConsensusResult {
                        query: res.query.to_owned(),
                        taxon: res.taxon.to_owned(),
                    })
                }
            };

            init
        },
    );

    match file.write_all(
        serde_json::to_string_pretty(&consensus_type_results)
            .unwrap()
            .as_bytes(),
    ) {
        Err(err) => panic!("{err}"),
        Ok(_) => (),
    };
}
