#![allow(dead_code)]

use blul_core::domain::{
    dtos::blast_builder::BlastBuilder,
    entities::execute_blastn::{ExecuteBlastn, ExecutionResponse},
};
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::Mutex,
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

/// `ExecuteBlastn` test double that records every FASTA payload it receives
/// and answers with a fixed outcome.
#[derive(Debug)]
pub struct MockBlastn {
    outcome: MockOutcome,
    pub calls: Mutex<Vec<String>>,
}

impl MockBlastn {
    pub fn new(outcome: MockOutcome) -> Self {
        Self {
            outcome,
            calls: Mutex::new(Vec::new()),
        }
    }

    pub fn succeeding_with(tsv: &str) -> Self {
        Self::new(MockOutcome::Success(tsv.to_string()))
    }

    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

impl ExecuteBlastn for MockBlastn {
    fn run(
        &self,
        query_sequences: String,
        _blast_config: BlastBuilder,
        _threads: usize,
    ) -> Result<ExecutionResponse, MappedErrors> {
        self.calls.lock().unwrap().push(query_sequences);

        match &self.outcome {
            MockOutcome::Success(out) => {
                Ok(ExecutionResponse::Success(out.clone()))
            }
            MockOutcome::Fail(err) => Ok(ExecutionResponse::Fail(err.clone())),
            MockOutcome::Error(err) => execution_err(err.clone()).as_error(),
        }
    }
}
