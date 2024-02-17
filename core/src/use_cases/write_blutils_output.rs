use crate::domain::dtos::{
    blast_builder::BlastBuilder,
    blutils_output::BlutilsOutput,
    consensus_result::{ConsensusResult, QueryWithConsensus},
};

use std::{fs::File, io::Write, path::PathBuf};
use tracing::info;

pub fn write_blutils_output(
    results: Vec<ConsensusResult>,
    config: Option<BlastBuilder>,
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

    let config = match config {
        Some(c) => Some(BlastBuilder {
            subject_reads: PathBuf::from(c.subject_reads)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            ..c
        }),
        None => None,
    };

    match file.write_all(
        serde_json::to_string_pretty(&BlutilsOutput {
            results: consensus_type_results,
            config,
        })
        .unwrap()
        .as_bytes(),
    ) {
        Err(err) => panic!("{err}"),
        Ok(_) => (),
    };
}
