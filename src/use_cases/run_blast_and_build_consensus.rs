use super::build_consensus_identities::build_consensus_identities;
use crate::domain::{
    dtos::blast_builder::BlastBuilder,
    entities::execute_step::{ExecuteStep, ExecutionResponse},
};

use clean_base::utils::errors::{execution_err, MappedErrors};
use log::{error, info, warn};
use rayon::prelude::*;
use std::{
    fs::{create_dir, remove_file, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::Path,
};

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
    // ? Load blast file input
    // ? ----------------------------------------------------------------------

    let input_file = match File::open(input_sequences) {
        Err(err) => return Err(execution_err(
            format!(
                "Unexpected error on try to initialize input file connection: {err}",
            ),
            None,
            None,
        )),
        Ok(res) => res,
    };

    // ? ----------------------------------------------------------------------
    // ? Load content
    // ? ----------------------------------------------------------------------

    let reader = BufReader::new(input_file);
    let mut lines = reader.lines();
    let mut source_sequences: Vec<String> = vec![];

    while let (Some(header), Some(sequence)) = (lines.next(), lines.next()) {
        if header.is_err() || sequence.is_err() {
            return Err(execution_err(
                String::from("Unexpected error on try to read query file."),
                None,
                None,
            ));
        }

        source_sequences.push(
            format!("{}\n{}\n", header.unwrap(), sequence.unwrap())
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
    info!("Output file could be found in {:?}", output_file);

    if output_file.exists() {
        if !overwrite {
            error!(
                "Could not overwrite existing file {:?} when overwrite option is `false`.", 
                output_file
            );

            panic!();
        }

        match remove_file(output_file.clone()) {
            Err(err) => panic!("Could not remove file given {}", err),
            Ok(_) => warn!("Output file overwritten: {:?}", output_file),
        };
    };

    // ? Processing sequences as chunks

    source_sequences
        .chunks(5)
        .enumerate()
        .par_bridge()
        .for_each(|(index, chunk)| {
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
                    panic!(
                        "Unexpected error on process chunk {}: {}",
                        index, err
                    );
                }
                ExecutionResponse::Success(res) => {
                    match write_tmp_file(res, output_file.as_path()) {
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

fn write_tmp_file(
    content: String,
    output_file: &Path,
) -> Result<bool, MappedErrors> {
    match OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(output_file)
        .unwrap()
        .write(content.as_bytes())
    {
        Err(err) => {
            error!("Unexpected error detected: {}", err);
            return Err(execution_err(
                String::from("Unexpected error detected on write temp file."),
                None,
                None,
            ));
        }
        Ok(_) => Ok(true),
    }
}
