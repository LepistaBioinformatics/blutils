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

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator,
};
use std::{
    fs::{create_dir_all, remove_file},
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
    // ? Build output file
    // ? ----------------------------------------------------------------------

    let mut out_dir_path = PathBuf::from(blast_out_file);
    out_dir_path.set_extension("out");
    let out_dir = out_dir_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    // Create every missing ancestor and surface failures (BUG-R33).
    if !out_dir.exists() {
        create_dir_all(&out_dir).map_err(|err| {
            execution_err(format!(
                "Could not create blast output directory {out_dir:?}: {err}"
            ))
        })?;
    }

    tracing::info!("");
    tracing::info!("Blast output file:");
    tracing::info!("\t{:?}", out_dir_path);
    tracing::info!("");

    if out_dir_path.exists() {
        if !overwrite {
            return execution_err(format!(
                "Could not overwrite existing file {:?} when overwrite option is `false`.",
                out_dir_path
            ))
            .as_error();
        }

        remove_file(out_dir_path.clone()).map_err(|err| {
            execution_err(format!(
                "Could not remove file {out_dir_path:?}: {err}"
            ))
        })?;

        tracing::warn!("Output file overwritten!");
    };

    // ? ----------------------------------------------------------------------
    // ? Process input sequences
    // ? ----------------------------------------------------------------------

    let chunk_size = 50;
    let (writer, file) = write_or_append_to_file(out_dir_path.as_path())?;
    let mut headers: Vec<String> = Vec::new();

    let source_sequences =
        input_sequences.sequence_content().map_err(|err| {
            execution_err(format!("Could not read input sequences: {err}"))
        })?;

    let sequences = source_sequences
        .to_owned()
        .into_iter()
        .map(|sequence| {
            headers.push(sequence.blast_header().to_owned());
            sequence
        })
        .collect::<Vec<Sequence>>();

    let chunks = sequences.chunks(chunk_size).collect::<Vec<_>>();

    //
    // BUG-R40: `threads` caps the whole run. Up to `threads` chunks run at
    // once on a dedicated pool, and the threads left over when there are fewer
    // chunks than threads go to each `blastn` call. Chunks used to run on the
    // global rayon pool while each `blastn` also got `-num_threads <threads>`.
    //
    let threads = threads.max(1);
    let concurrency = threads.min(chunks.len()).max(1);
    let threads_per_call = (threads / concurrency).max(1);

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(concurrency)
        .build()
        .map_err(|err| {
            execution_err(format!(
                "Could not build the BLAST thread pool: {err}"
            ))
        })?;

    let total_chunks = chunks.len();

    pool.install(|| {
        chunks.par_iter().enumerate().try_for_each(|(index, chunk)| {
            tracing::debug!(
                "Processing chunk {} of {}",
                index + 1,
                total_chunks
            );

            let response = blast_execution_repo
                .run(
                    chunk
                        .iter()
                        .map(|i| i.to_fasta())
                        .collect::<Vec<String>>()
                        .join(""),
                    blast_config.clone(),
                    threads_per_call,
                )
                .map_err(|err| {
                    execution_err(format!(
                        "Could not execute blastn on chunk {} of {}: {err}",
                        index + 1,
                        total_chunks
                    ))
                })?;

            match response {
                ExecutionResponse::Fail(err) => execution_err(format!(
                    "blastn failed on chunk {} of {}: {err}",
                    index + 1,
                    total_chunks
                ))
                .as_error(),
                ExecutionResponse::Success(res) => {
                    let handle = file.try_clone().map_err(|err| {
                        execution_err(format!(
                            "Could not open the BLAST output file {out_dir_path:?}: {err}"
                        ))
                    })?;

                    writer(res, handle).map_err(|err| {
                        execution_err(format!(
                            "Could not write chunk {} of {} to {out_dir_path:?}: {err}",
                            index + 1,
                            total_chunks
                        ))
                    })
                }
            }
        })
    })?;

    Ok(ParallelBlastOutput {
        output_file: out_dir_path.to_path_buf(),
        headers: Some(headers),
    })
}
