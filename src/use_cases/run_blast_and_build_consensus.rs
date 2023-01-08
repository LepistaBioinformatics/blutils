use super::{build_consensus_identities, run_parallel_blast};
use crate::domain::{
    dtos::blast_builder::BlastBuilder, entities::execute_step::ExecuteStep,
};

use clean_base::utils::errors::MappedErrors;
use std::path::Path;

pub fn run_blast_and_build_consensus(
    input_sequences: &str,
    input_taxonomies: &str,
    out_dir: &str,
    blast_config: BlastBuilder,
    blast_execution_repo: &dyn ExecuteStep,
    overwrite: &bool,
    threads: usize,
) -> Result<bool, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Execute parallel blast
    // ? ----------------------------------------------------------------------

    let output_file = match run_parallel_blast(
        input_sequences,
        out_dir,
        blast_config,
        blast_execution_repo,
        overwrite,
        threads,
    ) {
        Err(err) => return Err(err),
        Ok(res) => res,
    };

    // ? ----------------------------------------------------------------------
    // ? Build consensus
    // ? ----------------------------------------------------------------------

    match build_consensus_identities(
        output_file.as_path(),
        Path::new(input_taxonomies),
    ) {
        Err(err) => return Err(err),
        Ok(_) => (),
    };

    Ok(true)
}
