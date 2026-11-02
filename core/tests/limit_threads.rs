//! Runs in its own process: `limit_threads` configures process-wide pools.

use blul_core::use_cases::limit_threads;

// BUG-R40: `--threads` caps the global rayon pool (consensus, taxdump
// loaders) and the Polars pool, not only the BLAST step.
#[test]
fn limit_threads_caps_the_global_rayon_and_polars_pools() {
    std::env::remove_var("POLARS_MAX_THREADS");

    limit_threads(3).unwrap();

    assert_eq!(rayon::current_num_threads(), 3);
    assert_eq!(std::env::var("POLARS_MAX_THREADS").unwrap(), "3");
    assert!(
        limit_threads(2).is_err(),
        "the global pool is set only once"
    );
}
