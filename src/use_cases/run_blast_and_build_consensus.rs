use super::{
    build_consensus_identities, run_parallel_blast, ConsensusStrategy,
};
use crate::domain::{
    dtos::{blast_builder::BlastBuilder, blast_result::ConsensusResult},
    entities::execute_step::ExecuteStep,
};

use clean_base::utils::errors::{use_case_err, MappedErrors};
use std::path::Path;

/// Run parallel blast and build taxonomies consensus
pub fn run_blast_and_build_consensus(
    input_sequences: &str,
    input_taxonomies: &str,
    out_dir: &str,
    blast_config: BlastBuilder,
    blast_execution_repo: &dyn ExecuteStep,
    overwrite: &bool,
    threads: usize,
    strategy: ConsensusStrategy,
) -> Result<Vec<ConsensusResult>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Execute parallel blast
    // ? ----------------------------------------------------------------------

    let output = match run_parallel_blast(
        input_sequences,
        out_dir,
        blast_config.to_owned(),
        blast_execution_repo,
        overwrite,
        threads,
    ) {
        Err(err) => {
            return Err(use_case_err(
                String::from("Unexpected error on run parallel blast"),
                None,
                Some(err),
            ))
        }
        Ok(res) => res,
    };

    // ? ----------------------------------------------------------------------
    // ? Build consensus
    // ? ----------------------------------------------------------------------

    match build_consensus_identities(
        output,
        Path::new(input_taxonomies),
        blast_config,
        strategy,
    ) {
        Err(err) => {
            return Err(use_case_err(
                String::from("Unexpected error on build consensus taxonomy"),
                None,
                Some(err),
            ))
        }
        Ok(res) => Ok(res),
    }
}
