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
    path::PathBuf,
};

/// Run blast in parallel mode
///
/// This implementation target to saturate the host machine CPU utilization.
/// Simple blast usage not allows the full usage of these resource.
#[tracing::instrument(
    name = "Run Parallel Blast",
    skip(
        input_sequences,
        blast_out_file,
        blast_config,
        blast_execution_repo,
        overwrite,
        threads,
    )
)]
pub(super) fn run_parallel_blast(
    input_sequences: FileOrStdin,
    blast_out_file: &str,
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

    let mut out_dir_path = PathBuf::from(blast_out_file);
    out_dir_path.set_extension("out");
    let out_dir = out_dir_path.parent().unwrap();

    if !out_dir.exists() {
        let _ = create_dir(out_dir);
    }

    tracing::info!("");
    tracing::info!("Blast output file:");
    tracing::info!("\t{:?}", out_dir_path);
    tracing::info!("");

    if out_dir_path.exists() {
        if !overwrite {
            tracing::error!(
                "Could not overwrite existing file {:?} when overwrite option is `false`.", 
                out_dir_path
            );

            std::process::exit(1);
        }

        match remove_file(out_dir_path.clone()) {
            Err(err) => panic!("Could not remove file given {err}"),
            Ok(_) => tracing::warn!("Output file overwritten!"),
        };
    };

    // ? ----------------------------------------------------------------------
    // ? Process input sequences
    // ? ----------------------------------------------------------------------

    let chunk_size = 50;
    let (writer, file) = write_or_append_to_file(out_dir_path.as_path());
    let mut headers: Vec<String> = Vec::new();

    let source_sequences =
        input_sequences.sequence_content().map_err(|err| {
            panic!("Could not read input sequences: {err}");
        })?;

    source_sequences
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
            tracing::debug!(
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
                    panic!("Unexpected error detected on execute blast: {err}")
                }
                Ok(res) => res,
            };

            match response {
                ExecutionResponse::Fail(err) => {
                    panic!("Unexpected error on process chunk {index}: {err}");
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
        });

    Ok(ParallelBlastOutput {
        output_file: out_dir_path.to_path_buf(),
        headers: Some(headers),
    })
}
