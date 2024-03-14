use crate::{
    domain::{
        dtos::{
            blast_builder::BlastBuilder,
            file_or_stdin::{FileOrStdin, Sequence},
            parallel_blast_output::ParallelBlastOutput,
        },
        entities::execute_blastn::{ExecuteBlastn, ExecutionResponse},
    },
    use_cases::shared::{validate_blast_database, write_or_append_to_file},
};

use mycelium_base::utils::errors::MappedErrors;
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::{
    fs::{create_dir, remove_file},
    path::{Path, PathBuf},
};
use tracing::{debug, info, warn};

/// Run blast in parallel mode
///
/// This implementation target to saturate the host machine CPU utilization.
/// Simple blast usage not allows the full usage of these resource.
#[tracing::instrument(
    name = "Run Parallel Blast",
    skip(
        input_sequences,
        out_dir,
        blast_config,
        blast_execution_repo,
        overwrite,
        threads,
    )
)]
pub(super) fn run_parallel_blast(
    input_sequences: FileOrStdin,
    out_dir: &str,
    blast_config: BlastBuilder,
    blast_execution_repo: &dyn ExecuteBlastn,
    overwrite: &bool,
    threads: usize,
) -> Result<ParallelBlastOutput, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Validate blast database
    // ? ----------------------------------------------------------------------

    validate_blast_database(&PathBuf::from(
        blast_config.subject_reads.to_owned(),
    ))?;

    // ? ----------------------------------------------------------------------
    // ? Build thread pool
    // ? ----------------------------------------------------------------------

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();

    // ? ----------------------------------------------------------------------
    // ? Build output file
    // ? ----------------------------------------------------------------------

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

    // ? ----------------------------------------------------------------------
    // ? Process input sequences
    // ? ----------------------------------------------------------------------

    let chunk_size = 50;
    let (writer, file) = write_or_append_to_file(output_file.as_path());
    let mut headers: Vec<String> = Vec::new();

    match input_sequences.content() {
        Ok(source_sequences) => source_sequences
            .to_owned()
            .into_iter()
            .map(|sequence| {
                headers.push(sequence.blast_header().to_owned());
                sequence
            })
            .collect::<Vec<Sequence>>()
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
                    blast_execution_repo.run(
                        chunk
                            .into_iter()
                            .map(|i| i.to_fasta())
                            .collect::<Vec<String>>()
                            .join(""),
                        blast_config.clone(),
                        threads,
                    )
                }) {
                    Err(err) => {
                        panic!(
                            "Unexpected error detected on execute blast: {err}"
                        )
                    }
                    Ok(res) => res,
                };

                match response {
                    ExecutionResponse::Fail(err) => {
                        panic!(
                            "Unexpected error on process chunk {index}: {err}"
                        );
                    }
                    ExecutionResponse::Success(res) => {
                        match writer(
                        res,
                        file.try_clone().expect(
                            "Unexpected error detected on write blast result",
                        ),
                    ) {
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
            }),
        Err(err) => panic!("{err}"),
    };

    Ok(ParallelBlastOutput {
        output_file,
        headers: Some(headers),
    })
}
