mod run_parallel_blast;

use run_parallel_blast::*;

use super::{build_consensus_identities, write_blutils_output, OutputFormat};
use crate::domain::{
    dtos::{
        blast_builder::BlastBuilder, consensus_strategy::ConsensusStrategy,
        file_or_stdin::FileOrStdin,
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
    input_sequences: FileOrStdin,
    input_taxonomies: &str,
    blast_out_file: &str,
    blutils_out_file: Option<String>,
    blast_config: BlastBuilder,
    blast_execution_repo: &dyn ExecuteBlastn,
    overwrite: &bool,
    threads: usize,
    strategy: ConsensusStrategy,
    use_taxid: Option<bool>,
    out_format: OutputFormat,
) -> Result<bool, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Execute parallel blast
    // ? -----------------------------------------------------------------------

    let output = run_parallel_blast(
        input_sequences,
        blast_out_file,
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

    if let Err(err) = write_blutils_output(
        blast_output.to_owned(),
        Some(blast_config),
        blutils_out_file,
        out_format,
    ) {
        return Err(err);
    };

    Ok(true)
}
