use crate::{
    domain::{
        dtos::{
            blast_builder::BlastBuilder,
            parallel_blast_output::ParallelBlastOutput,
        },
        entities::execute_blastn::{ExecuteBlastn, ExecutionResponse},
    },
    use_cases::shared::write_or_append_to_file,
};

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use rayon::prelude::*;
use std::{
    fs::{create_dir, remove_file, File},
    io::{BufRead, BufReader},
    path::Path,
};
use tracing::{debug, info, warn};

/// Run blast in parallel mode
///
/// This implementation target to saturate the host machine CPU utilization.
/// Simple blast usage not allows the full usage of these resource.
#[tracing::instrument(name = "Run Parallel Blast")]
pub(super) fn run_parallel_blast(
    input_sequences: &str,
    out_dir: &str,
    blast_config: BlastBuilder,
    blast_execution_repo: &dyn ExecuteBlastn,
    overwrite: &bool,
    threads: usize,
) -> Result<ParallelBlastOutput, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Load blast file input
    // ? ----------------------------------------------------------------------

    let input_file = match File::open(input_sequences) {
        Err(err) => return execution_err(
            format!(
                "Unexpected error on try to initialize input file connection: {err}",
            )
        ).as_error(),
        Ok(res) => res,
    };

    // ? ----------------------------------------------------------------------
    // ? Load content
    // ? ----------------------------------------------------------------------

    let reader = BufReader::new(input_file);
    let mut lines = reader.lines();
    let mut source_sequences: Vec<String> = vec![];
    let mut headers: Vec<String> = Vec::new();

    while let (Some(header), Some(sequence)) = (lines.next(), lines.next()) {
        if header.is_err() {
            return execution_err(format!(
                "Unexpected error on try to read sequence header of sequences file: {}",
                header.unwrap_err(),
            ))
            .as_error();
        }

        if sequence.is_err() {
            return execution_err(format!(
                "Unexpected error on try to read nucleotide of sequences file: {}",
                sequence.unwrap_err(),
            ))
            .as_error();
        }

        let header = header.unwrap();

        headers.push(header.replace(">", "").to_owned());

        source_sequences.push(
            format!("{}\n{}\n", header, sequence.unwrap())
                .as_str()
                .to_owned(),
        );
    }

    // ? ----------------------------------------------------------------------
    // ? Execute parallel BlastN and persist output
    // ? ----------------------------------------------------------------------

    // ? Build thread pool

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();

    // ? Build output file

    let out_dir_path = Path::new(out_dir);

    if !out_dir_path.exists() {
        let _ = create_dir(out_dir_path);
    }

    let output_file = out_dir_path.join("blast.out");
    info!("");
    info!("Blast output file:");
    info!("\t{:?}", output_file);
    info!("");

    if output_file.exists() {
        if !overwrite {
            panic!(
                "Could not overwrite existing file {:?} when overwrite option is `false`.", 
                output_file
            );
        }

        match remove_file(output_file.clone()) {
            Err(err) => panic!("Could not remove file given {}", err),
            Ok(_) => warn!("Output file overwritten!"),
        };
    };

    // ? Processing sequences as chunks

    let chunk_size = source_sequences.len() / threads;
    let chunk_size = if chunk_size == 0 { 1 } else { chunk_size };
    debug!("Total Sequences: {}", source_sequences.len());
    debug!("Number of Threads: {}", threads);
    debug!("Chunk Size: {}", chunk_size);

    source_sequences
        .chunks(chunk_size)
        .enumerate()
        .par_bridge()
        .for_each(|(index, chunk)| {
            debug!(
                "Processing chunk {} of {:?}",
                index + 1,
                source_sequences.len() / chunk_size
            );

            let response = match pool.install(|| {
                blast_execution_repo.run(chunk.join(""), blast_config.clone())
            }) {
                Err(err) => {
                    panic!("Unexpected error detected on execute blast: {err}")
                }
                Ok(res) => res,
            };

            match response {
                ExecutionResponse::Fail(err) => {
                    panic!("Unexpected error on process chunk {index}: {err}");
                }
                ExecutionResponse::Success(res) => {
                    match write_or_append_to_file(res, output_file.as_path()) {
                        Err(err) => {
                            panic!(
                                "Unexpected error on persist chunk {}: {}",
                                index, err
                            )
                        }
                        Ok(_) => (),
                    };
                }
            };
        });

    Ok(ParallelBlastOutput {
        output_file,
        headers,
    })
}
