use crate::{
    domain::dtos::{
        blast_builder::BlastBuilder,
        blutils_output::BlutilsOutput,
        consensus_result::{ConsensusResult, QueryWithConsensus},
    },
    use_cases::shared::write_or_append_to_file,
};

use mycelium_base::utils::errors::MappedErrors;
use serde::{Deserialize, Serialize};
use std::{
    fs::{remove_file, File},
    io::Write,
    path::PathBuf,
};
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "camelCase")]
pub enum OutputFormat {
    Json,
    Jsonl,
}

pub fn write_blutils_output(
    results: Vec<ConsensusResult>,
    config: Option<BlastBuilder>,
    blutils_out_file: Option<String>,
    out_format: OutputFormat,
) -> Result<(), MappedErrors> {
    let blutils_out_file = match blutils_out_file {
        Some(file) => {
            let mut path = PathBuf::from(file);
            path.set_extension("json");

            info!("");
            info!("Blutils output file:");
            info!("\t{:?}", path);
            info!("");

            if path.exists() {
                match remove_file(path.clone()) {
                    Err(err) => panic!("Could not remove file given {err}"),
                    Ok(_) => warn!("Output file overwritten!"),
                };
            };

            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    match std::fs::create_dir_all(parent) {
                        Err(err) => {
                            panic!("Could not create directory given {err}")
                        }
                        Ok(_) => (),
                    };
                }
            }

            Some(path)
        }
        None => None,
    };

    let run_id = match config.to_owned() {
        Some(c) => c.run_id,
        None => Uuid::new_v4(),
    };

    let mut consensus_type_results = results.iter().fold(
        Vec::<QueryWithConsensus>::new(),
        |mut init, record| {
            match record {
                ConsensusResult::NoConsensusFound(res) => {
                    init.push(QueryWithConsensus {
                        query: res.query.to_owned(),
                        taxon: None,
                        run_id: Some(run_id),
                    });
                }
                ConsensusResult::ConsensusFound(res) => {
                    init.push(QueryWithConsensus {
                        query: res.query.to_owned(),
                        taxon: res.taxon.to_owned(),
                        run_id: Some(run_id),
                    })
                }
            };

            init
        },
    );

    consensus_type_results.sort_by(|a, b| a.query.cmp(&b.query));

    let config = match config {
        Some(config) => Some(BlastBuilder {
            subject_reads: PathBuf::from(config.subject_reads)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            ..config
        }),
        None => None,
    };

    match out_format {
        OutputFormat::Json => {
            if let Some(output_file) = blutils_out_file {
                let mut file = match File::create(output_file.to_owned()) {
                    Err(err) => panic!(
                        "Error on persist output results into {}: {err}",
                        output_file.as_os_str().to_str().unwrap()
                    ),
                    Ok(res) => res,
                };

                match file.write_all(
                    serde_json::to_string_pretty(&BlutilsOutput {
                        results: consensus_type_results,
                        config,
                    })
                    .unwrap()
                    .as_bytes(),
                ) {
                    Err(err) => panic!(
                        "Unexpected error on write config to output file: {err}"
                    ),
                    Ok(_) => (),
                };

                Ok(())
            } else {
                if let Err(err) = serde_json::to_writer(
                    std::io::stdout().lock(),
                    &BlutilsOutput {
                        results: consensus_type_results,
                        config,
                    },
                ) {
                    panic!("Unexpected error on write JSON output: {err}");
                } else {
                    Ok(())
                }
            }
        }
        OutputFormat::Jsonl => {
            if let Some(output_file) = blutils_out_file {
                let (writer, file) =
                    write_or_append_to_file(output_file.as_path());

                writer(
                    serde_json::to_string(&config).unwrap() + "\n",
                    file.try_clone().expect(
                        "Unexpected error detected on write blast result",
                    ),
                )?;

                for record in &consensus_type_results {
                    match writer(
                        serde_json::to_string(&record).unwrap() + "\n",
                        file.try_clone().expect(
                            "Unexpected error detected on write blast result",
                        ),
                    ) {
                        Err(err) => {
                            panic!("Unexpected error on write JSONL output file: {err}",)
                        }
                        Ok(_) => (),
                    }
                }

                Ok(())
            } else {
                let mut stdout = std::io::stdout();

                if let Err(err) = serde_json::to_writer(stdout.lock(), &config)
                {
                    panic!("Unexpected error on write JSONL output: {err}");
                }

                stdout.write(b"\n").unwrap();

                for record in &consensus_type_results {
                    if let Err(err) =
                        serde_json::to_writer(stdout.lock(), &record)
                    {
                        panic!("Unexpected error on write JSONL output: {err}");
                    }

                    stdout.write(b"\n").unwrap();
                }

                Ok(())
            }
        }
    }
}
