mod run_parallel_blast;

use run_parallel_blast::*;

use super::{build_consensus_identities, write_blutils_output};
use crate::domain::{
    dtos::{
        blast_builder::BlastBuilder, consensus_strategy::ConsensusStrategy,
    },
    entities::execute_blastn::ExecuteBlastn,
};

use mycelium_base::utils::errors::MappedErrors;
use std::path::Path;

/// Run parallel blast and build taxonomies consensus
#[tracing::instrument(
    name = "Run Blast with Consensus",
    skip(blast_execution_repo, blast_config, overwrite, strategy, use_taxid)
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
    use_taxid: Option<bool>,
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
        blast_config.taxon.to_owned(),
        strategy,
        use_taxid,
    )?;

    write_blutils_output(
        blast_output.to_owned(),
        Some(blast_config),
        Path::new(out_dir).to_path_buf(),
    );

    Ok(true)
}
