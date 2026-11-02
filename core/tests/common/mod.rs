#![allow(dead_code)]

use blul_core::domain::{
    dtos::blast_builder::BlastBuilder,
    entities::execute_blastn::{ExecuteBlastn, ExecutionResponse},
};
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    thread::sleep,
    time::Duration,
};

/// Absolute path of a file under `core/tests/fixtures/`.
pub fn fixture(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(relative)
}

/// Creates the minimal files `validate_blast_database` checks for
/// (`<name>.00.nsq` and `taxdb.btd`) and returns the database path to pass as
/// `subject_reads`.
pub fn fake_blast_db(dir: &Path) -> PathBuf {
    File::create(dir.join("db.00.nsq")).unwrap();
    File::create(dir.join("taxdb.btd")).unwrap();
    dir.join("db.fna")
}

#[derive(Debug, Clone)]
pub enum MockOutcome {
    Success(String),
    Fail(String),
    Error(String),
}

/// `ExecuteBlastn` test double that records every FASTA payload it receives,
/// the `threads` value of each call and the peak number of concurrent calls,
/// and answers with a fixed outcome.
#[derive(Debug)]
pub struct MockBlastn {
    outcome: MockOutcome,
    delay: Duration,
    in_flight: AtomicUsize,
    pub max_in_flight: AtomicUsize,
    pub calls: Mutex<Vec<String>>,
    pub threads: Mutex<Vec<usize>>,
}

impl MockBlastn {
    pub fn new(outcome: MockOutcome) -> Self {
        Self {
            outcome,
            delay: Duration::ZERO,
            in_flight: AtomicUsize::new(0),
            max_in_flight: AtomicUsize::new(0),
            calls: Mutex::new(Vec::new()),
            threads: Mutex::new(Vec::new()),
        }
    }

    pub fn succeeding_with(tsv: &str) -> Self {
        Self::new(MockOutcome::Success(tsv.to_string()))
    }

    /// Keeps each call in flight for `delay`, so concurrent calls overlap.
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }

    /// Peak number of host threads requested at once: the largest `threads`
    /// value received times the peak number of concurrent calls.
    pub fn peak_threads(&self) -> usize {
        let max_threads = self
            .threads
            .lock()
            .unwrap()
            .iter()
            .copied()
            .max()
            .unwrap_or(0);
        max_threads * self.max_in_flight.load(Ordering::SeqCst)
    }
}

impl ExecuteBlastn for MockBlastn {
    fn run(
        &self,
        query_sequences: String,
        _blast_config: BlastBuilder,
        threads: usize,
    ) -> Result<ExecutionResponse, MappedErrors> {
        let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_in_flight.fetch_max(now, Ordering::SeqCst);
        sleep(self.delay);
        self.in_flight.fetch_sub(1, Ordering::SeqCst);

        self.calls.lock().unwrap().push(query_sequences);
        self.threads.lock().unwrap().push(threads);

        match &self.outcome {
            MockOutcome::Success(out) => {
                Ok(ExecutionResponse::Success(out.clone()))
            }
            MockOutcome::Fail(err) => Ok(ExecutionResponse::Fail(err.clone())),
            MockOutcome::Error(err) => execution_err(err.clone()).as_error(),
        }
    }
}
