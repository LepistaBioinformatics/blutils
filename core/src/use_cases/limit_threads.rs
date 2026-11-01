use mycelium_base::utils::errors::{execution_err, MappedErrors};

/// Caps the threads of the whole process at `threads`.
///
/// BUG-R40: besides the BLAST chunks, the consensus step and the taxdump
/// loaders use the global rayon pool and Polars its own pool, both sized to
/// every visible core by default. Must run once, before any parallel work.
/// An explicit `POLARS_MAX_THREADS` in the environment is kept.
pub fn limit_threads(threads: usize) -> Result<(), MappedErrors> {
    let threads = threads.max(1);

    if std::env::var_os("POLARS_MAX_THREADS").is_none() {
        std::env::set_var("POLARS_MAX_THREADS", threads.to_string());
    }

    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()
        .map_err(|err| {
            execution_err(format!("Could not limit the thread pool: {err}"))
        })
}
