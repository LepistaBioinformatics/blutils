mod run_parallel_blast;

use run_parallel_blast::*;

use super::build_consensus_identities;
use crate::domain::{
    dtos::{
        blast_builder::BlastBuilder,
        consensus_result::{ConsensusResult, QueryWithConsensus},
        consensus_strategy::ConsensusStrategy,
    },
    entities::execute_blastn::ExecuteBlastn,
};

use mycelium_base::utils::errors::MappedErrors;
use serde::Serialize;
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
use tracing::info;

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

    write_json_output(
        blast_output.to_owned(),
        blast_config,
        Path::new(out_dir).to_path_buf(),
    );

    Ok(true)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BlutilsOutput {
    results: Vec<QueryWithConsensus>,
    config: BlastBuilder,
}

fn write_json_output(
    results: Vec<ConsensusResult>,
    config: BlastBuilder,
    out_dir: PathBuf,
) {
    let output_file = out_dir.join("blutils.consensus.json");
    info!("");
    info!("Blutils output file:");
    info!("\t{:?}", output_file);
    info!("");

    let mut file = match File::create(output_file) {
        Err(err) => panic!("{err}"),
        Ok(res) => res,
    };

    let mut consensus_type_results = results.iter().fold(
        Vec::<QueryWithConsensus>::new(),
        |mut init, record| {
            match record {
                ConsensusResult::NoConsensusFound(res) => {
                    init.push(QueryWithConsensus {
                        query: res.query.to_owned(),
                        taxon: None,
                    });
                }
                ConsensusResult::ConsensusFound(res) => {
                    init.push(QueryWithConsensus {
                        query: res.query.to_owned(),
                        taxon: res.taxon.to_owned(),
                    })
                }
            };

            init
        },
    );

    consensus_type_results.sort_by(|a, b| a.query.cmp(&b.query));

    match file.write_all(
        serde_json::to_string_pretty(&BlutilsOutput {
            results: consensus_type_results,
            config: BlastBuilder {
                subject_reads: PathBuf::from(config.subject_reads)
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
                ..config
            },
        })
        .unwrap()
        .as_bytes(),
    ) {
        Err(err) => panic!("{err}"),
        Ok(_) => (),
    };
}
