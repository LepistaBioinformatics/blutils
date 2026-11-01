use crate::{
    domain::dtos::{
        blast_builder::BlastBuilder,
        blutils_output::BlutilsOutput,
        consensus_result::{ConsensusResult, QueryWithConsensus},
    },
    use_cases::shared::write_or_append_to_file,
};

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::{Deserialize, Serialize};
use std::{
    fs::{remove_file, File},
    io::{BufWriter, Write},
    path::PathBuf,
};
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "camelCase")]
pub enum OutputFormat {
    /// JSON format
    Json,

    /// JSONL format
    Jsonl,

    /// Yaml format
    Yaml,
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
            match out_format {
                OutputFormat::Jsonl => {
                    path.set_extension("jsonl");
                }
                OutputFormat::Json => {
                    path.set_extension("json");
                }
                OutputFormat::Yaml => {
                    path.set_extension("yaml");
                }
            }

            info!("");
            info!("Blutils output file:");
            info!("\t{:?}", path);
            info!("");

            // BUG-R43: file-system errors return `Err` instead of panicking.
            if path.exists() {
                remove_file(&path).map_err(|err| {
                    execution_err(format!(
                        "Could not overwrite the output file {path:?}: {err}"
                    ))
                })?;
                warn!("Output file overwritten!");
            };

            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|err| {
                        execution_err(format!(
                            "Could not create the output directory {parent:?}: {err}"
                        ))
                    })?;
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
                        // Keep a run id already carried by the result
                        // (BUG-R27).
                        run_id: res.run_id.or(Some(run_id)),
                    })
                }
            };

            init
        },
    );

    consensus_type_results.sort_by(|a, b| a.query.cmp(&b.query));

    let config = match config {
        Some(config) => Some(BlastBuilder {
            subject_reads: PathBuf::from(&config.subject_reads)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| config.subject_reads.to_owned()),
            ..config
        }),
        None => None,
    };

    let output = BlutilsOutput {
        results: consensus_type_results,
        config,
    };

    let create = |path: &PathBuf| {
        File::create(path).map(BufWriter::new).map_err(|err| {
            execution_err(format!(
                "Could not create the output file {path:?}: {err}"
            ))
        })
    };
    let write_err = |target: &str, err: &dyn std::fmt::Display| {
        execution_err(format!("Could not write the {target} output: {err}"))
    };

    match out_format {
        OutputFormat::Json => match blutils_out_file {
            Some(output_file) => {
                let mut file = create(&output_file)?;
                serde_json::to_writer_pretty(&mut file, &output)
                    .map_err(|err| write_err("JSON", &err))?;
                file.flush().map_err(|err| write_err("JSON", &err))
            }
            None => serde_json::to_writer(std::io::stdout().lock(), &output)
                .map_err(|err| write_err("JSON", &err)),
        },
        OutputFormat::Jsonl => {
            // Without config no line is written: a bare `null` line is not a
            // valid record (BUG-R09).
            let mut lines = Vec::new();
            if let Some(config) = &output.config {
                lines.push(
                    serde_json::to_string(config)
                        .map_err(|err| write_err("JSONL", &err))?,
                );
            }
            for record in &output.results {
                lines.push(
                    serde_json::to_string(record)
                        .map_err(|err| write_err("JSONL", &err))?,
                );
            }

            match blutils_out_file {
                Some(output_file) => {
                    let (writer, file) =
                        write_or_append_to_file(output_file.as_path())?;

                    for line in lines {
                        let handle = file
                            .try_clone()
                            .map_err(|err| write_err("JSONL", &err))?;
                        writer(line + "\n", handle)?;
                    }

                    Ok(())
                }
                None => {
                    let mut stdout = std::io::stdout().lock();

                    for line in lines {
                        stdout
                            .write_all((line + "\n").as_bytes())
                            .map_err(|err| write_err("JSONL", &err))?;
                    }

                    Ok(())
                }
            }
        }
        OutputFormat::Yaml => match blutils_out_file {
            Some(output_file) => {
                let mut file = create(&output_file)?;
                serde_yaml_ng::to_writer(&mut file, &output)
                    .map_err(|err| write_err("YAML", &err))?;
                file.flush().map_err(|err| write_err("YAML", &err))
            }
            None => serde_yaml_ng::to_writer(std::io::stdout().lock(), &output)
                .map_err(|err| write_err("YAML", &err)),
        },
    }
}
