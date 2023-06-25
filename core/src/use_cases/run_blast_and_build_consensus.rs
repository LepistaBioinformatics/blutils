use super::{
    build_consensus_identities::build_consensus_identities,
    run_parallel_blast::run_parallel_blast, ConsensusStrategy,
};
use crate::domain::{
    dtos::{
        blast_builder::BlastBuilder,
        blast_result::{BlastQueryConsensusResult, ConsensusResult},
    },
    entities::execute_step::ExecuteStep,
};

use clean_base::utils::errors::MappedErrors;
use log::info;
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

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
) -> Result<bool, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Execute parallel blast
    // ? ----------------------------------------------------------------------

    let output = run_parallel_blast(
        input_sequences,
        out_dir,
        blast_config.to_owned(),
        blast_execution_repo,
        overwrite,
        threads,
    )?;

    // ? ----------------------------------------------------------------------
    // ? Build consensus
    // ? ----------------------------------------------------------------------

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
        Vec::<BlastQueryConsensusResult>::new(),
        |mut init, record| {
            match record {
                ConsensusResult::NoConsensusFound(res) => {
                    init.push(BlastQueryConsensusResult {
                        query: res.query.to_owned(),
                        taxon: None,
                    });
                }
                ConsensusResult::ConsensusFound(res) => {
                    init.push(BlastQueryConsensusResult {
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
