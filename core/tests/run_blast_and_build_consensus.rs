mod common;

use blul_core::{
    domain::dtos::{
        blast_builder::BlastBuilder, consensus_strategy::ConsensusStrategy,
        file_or_stdin::FileOrStdin, taxon::Taxon,
    },
    use_cases::{run_blast_and_build_consensus, OutputFormat},
};
use common::{fake_blast_db, fixture, MockBlastn, MockOutcome};
use mycelium_base::utils::errors::MappedErrors;
use serde_json::Value;
use std::{
    fs::{read_to_string, write},
    path::PathBuf,
    str::FromStr,
    time::Duration,
};
use tempfile::TempDir;

/// Writes a FASTA file with one `ACGT` record per header.
fn write_fasta(dir: &TempDir, headers: &[String]) -> PathBuf {
    let path = dir.path().join("query.fna");
    let content = headers
        .iter()
        .map(|header| format!(">{header}\nACGT\n"))
        .collect::<String>();
    write(&path, content).unwrap();
    path
}

fn numbered_headers(count: usize) -> Vec<String> {
    (1..=count).map(|i| format!("seq{i:03}")).collect()
}

fn basic_tsv() -> String {
    read_to_string(fixture("blast/basic.tsv")).unwrap()
}

fn basic_queries() -> Vec<String> {
    [
        "q1-single-100",
        "q2-single-98",
        "q3-multi-agree",
        "q4-multi-lca-genus",
    ]
    .iter()
    .map(|query| query.to_string())
    .collect()
}

struct Run {
    dir: TempDir,
    query: PathBuf,
    database: PathBuf,
    overwrite: bool,
}

impl Run {
    /// A run over a valid fake BLAST database and a FASTA with `headers`.
    fn new(headers: &[String]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let query = write_fasta(&dir, headers);
        let database = fake_blast_db(dir.path());

        Run {
            dir,
            query,
            database,
            overwrite: false,
        }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    fn execute(&self, mock: &MockBlastn) -> Result<bool, MappedErrors> {
        self.execute_with_threads(mock, 2)
    }

    fn execute_with_threads(
        &self,
        mock: &MockBlastn,
        threads: usize,
    ) -> Result<bool, MappedErrors> {
        run_blast_and_build_consensus(
            FileOrStdin::from_str(self.query.to_str().unwrap()).unwrap(),
            fixture("taxonomies/basic.blutils.json").to_str().unwrap(),
            self.path("blast.tsv").to_str().unwrap(),
            Some(self.path("result").to_str().unwrap().to_string()),
            BlastBuilder::default(
                self.database.to_str().unwrap(),
                Taxon::Bacteria,
            ),
            mock,
            &self.overwrite,
            threads,
            ConsensusStrategy::Cautious,
            None,
            OutputFormat::Json,
            None,
        )
    }

    fn blast_out(&self) -> String {
        read_to_string(self.path("blast.out")).unwrap()
    }

    fn result(&self) -> Value {
        serde_json::from_str(&read_to_string(self.path("result.json")).unwrap())
            .unwrap()
    }
}

fn result_queries(result: &Value) -> Vec<String> {
    result["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["query"].as_str().unwrap().to_string())
        .collect()
}

fn result_for<'a>(result: &'a Value, query: &str) -> &'a Value {
    result["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["query"] == query)
        .unwrap_or_else(|| panic!("query {query} not in results"))
}

/// Number of FASTA records in each payload the mock received, sorted
/// (chunks are dispatched in parallel, so call order is not deterministic).
fn records_per_call(mock: &MockBlastn) -> Vec<usize> {
    let mut counts = mock
        .calls
        .lock()
        .unwrap()
        .iter()
        .map(|payload| payload.matches('>').count())
        .collect::<Vec<_>>();
    counts.sort();
    counts
}

fn error_message(result: Result<bool, MappedErrors>) -> String {
    format!("{}", result.unwrap_err())
}

#[test]
fn run_blast_and_build_consensus_missing_nsq_returns_err_before_blast() {
    let run = Run::new(&basic_queries());
    std::fs::remove_file(run.path("db.00.nsq")).unwrap();
    let mock = MockBlastn::succeeding_with(&basic_tsv());

    let message = error_message(run.execute(&mock));

    assert!(message.contains("Blast database not found"), "{message}");
    assert_eq!(mock.call_count(), 0);
    assert!(!run.path("blast.out").exists());
    assert!(!run.path("result.json").exists());
}

#[test]
fn run_blast_and_build_consensus_missing_taxdb_returns_err_before_blast() {
    let run = Run::new(&basic_queries());
    std::fs::remove_file(run.path("taxdb.btd")).unwrap();
    let mock = MockBlastn::succeeding_with(&basic_tsv());

    let message = error_message(run.execute(&mock));

    assert!(message.contains("Taxdb not found"), "{message}");
    assert_eq!(mock.call_count(), 0);
    assert!(!run.path("blast.out").exists());
}

#[test]
fn run_blast_and_build_consensus_blast_out_file_extension_forced_to_out() {
    let run = Run::new(&basic_queries());
    let mock = MockBlastn::succeeding_with(&basic_tsv());

    assert!(run.execute(&mock).unwrap());

    assert!(run.path("blast.out").is_file());
    assert!(!run.path("blast.tsv").exists());
}

#[test]
fn run_blast_and_build_consensus_overwrite_true_replaces_existing_out() {
    let mut run = Run::new(&basic_queries());
    run.overwrite = true;
    write(run.path("blast.out"), "stale\tcontent\n").unwrap();
    let mock = MockBlastn::succeeding_with(&basic_tsv());

    assert!(run.execute(&mock).unwrap());

    assert_eq!(run.blast_out(), basic_tsv());
}

#[test]
fn run_blast_and_build_consensus_overwrite_true_without_existing_out_succeeds()
{
    let mut run = Run::new(&basic_queries());
    run.overwrite = true;
    let mock = MockBlastn::succeeding_with(&basic_tsv());

    assert!(run.execute(&mock).unwrap());

    assert_eq!(run.blast_out(), basic_tsv());
}

#[test]
fn run_blast_and_build_consensus_120_sequences_split_into_chunks_of_50() {
    let headers = numbered_headers(120);
    let run = Run::new(&headers);
    let mock = MockBlastn::succeeding_with("");

    assert!(run.execute(&mock).unwrap());

    assert_eq!(mock.call_count(), 3);
    assert_eq!(records_per_call(&mock), vec![20, 50, 50]);

    let mut sent = mock
        .calls
        .lock()
        .unwrap()
        .iter()
        .flat_map(|payload| {
            payload
                .lines()
                .filter(|line| line.starts_with('>'))
                .map(|line| line[1..].to_string())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    sent.sort();
    assert_eq!(sent, headers);
}

#[test]
fn run_blast_and_build_consensus_payload_is_fasta_with_full_header() {
    let run = Run::new(&["q1-single-100 some description".to_string()]);
    let mock = MockBlastn::succeeding_with("");

    assert!(run.execute(&mock).unwrap());

    assert_eq!(
        *mock.calls.lock().unwrap(),
        vec![">q1-single-100 some description\nACGT\n".to_string()]
    );
}

#[test]
fn run_blast_and_build_consensus_single_chunk_success_written_verbatim_to_out()
{
    let run = Run::new(&basic_queries());
    let mock = MockBlastn::succeeding_with(&basic_tsv());

    assert!(run.execute(&mock).unwrap());

    assert_eq!(mock.call_count(), 1);
    assert_eq!(run.blast_out(), basic_tsv());
}

#[test]
fn run_blast_and_build_consensus_successes_concatenated_into_out() {
    let run = Run::new(&numbered_headers(60));
    let mock = MockBlastn::succeeding_with(&basic_tsv());

    assert!(run.execute(&mock).unwrap());

    assert_eq!(mock.call_count(), 2);
    assert_eq!(run.blast_out(), basic_tsv().repeat(2));
}

#[test]
fn run_blast_and_build_consensus_headers_use_first_token_of_fasta_header() {
    let run = Run::new(&[
        "q1-single-100 Bacillus subtilis 16S".to_string(),
        "q9-no-hit some description".to_string(),
    ]);
    let mock = MockBlastn::succeeding_with(&basic_tsv());

    assert!(run.execute(&mock).unwrap());

    let result = run.result();
    assert!(result_queries(&result).contains(&"q9-no-hit".to_string()));
    assert_eq!(result_for(&result, "q9-no-hit")["taxon"], Value::Null);
    assert_eq!(
        result_for(&result, "q1-single-100")["taxon"]["identifier"],
        "bacillus-subtilis"
    );
}

#[test]
fn run_blast_and_build_consensus_empty_blast_output_reports_every_query_without_taxon(
) {
    let run = Run::new(&numbered_headers(3));
    let mock = MockBlastn::succeeding_with("");

    assert!(run.execute(&mock).unwrap());

    let result = run.result();
    assert_eq!(result_queries(&result), numbered_headers(3));
    for query in numbered_headers(3) {
        assert_eq!(result_for(&result, &query)["taxon"], Value::Null);
    }
}

// BUG-R43: a `blastn` failure used to `panic!` inside the worker pool.
#[test]
fn run_blast_and_build_consensus_blast_fail_returns_err_with_blastn_message() {
    let run = Run::new(&basic_queries());
    let mock = MockBlastn::new(MockOutcome::Fail("blastn: bad db".to_string()));

    let message = error_message(run.execute(&mock));

    assert!(
        message.contains("blastn failed on chunk 1 of 1"),
        "{message}"
    );
    assert!(message.contains("blastn: bad db"), "{message}");
}

// BUG-R43: an error launching `blastn` used to `panic!` inside the pool.
#[test]
fn run_blast_and_build_consensus_blast_error_returns_err_with_cause() {
    let run = Run::new(&basic_queries());
    let mock =
        MockBlastn::new(MockOutcome::Error("blastn not found".to_string()));

    let message = error_message(run.execute(&mock));

    assert!(
        message.contains("Could not execute blastn on chunk 1 of 1"),
        "{message}"
    );
    assert!(message.contains("blastn not found"), "{message}");
}

#[test]
fn run_blast_and_build_consensus_end_to_end_writes_expected_json() {
    let run = Run::new(&basic_queries());
    let mock = MockBlastn::succeeding_with(&basic_tsv());

    assert!(run.execute(&mock).unwrap());

    let result = run.result();
    assert_eq!(result_queries(&result), basic_queries());

    let config = &result["config"];
    assert_eq!(config["isConfig"], true);
    assert_eq!(config["subjectReads"], "db.fna");
    assert_eq!(config["taxon"], "bacteria");
    for record in result["results"].as_array().unwrap() {
        assert_eq!(record["runId"], config["runId"]);
    }

    let q1 = &result_for(&result, "q1-single-100")["taxon"];
    assert_eq!(q1["reachedRank"], "species");
    assert_eq!(q1["identifier"], "bacillus-subtilis");
    assert_eq!(q1["singleMatch"], true);

    let q2 = &result_for(&result, "q2-single-98")["taxon"];
    assert_eq!(q2["reachedRank"], "genus");
    assert_eq!(q2["identifier"], "bacillus");

    let q3 = &result_for(&result, "q3-multi-agree")["taxon"];
    assert_eq!(q3["reachedRank"], "species");
    assert_eq!(q3["identifier"], "bacillus-subtilis");

    let q4 = &result_for(&result, "q4-multi-lca-genus")["taxon"];
    assert_eq!(q4["reachedRank"], "family");
    assert_eq!(q4["identifier"], "bacillaceae");
}

fn execute_with_blast_out(
    run: &Run,
    blast_out_file: &str,
    mock: &MockBlastn,
) -> Result<bool, MappedErrors> {
    run_blast_and_build_consensus(
        FileOrStdin::from_str(run.query.to_str().unwrap()).unwrap(),
        fixture("taxonomies/basic.blutils.json").to_str().unwrap(),
        run.path(blast_out_file).to_str().unwrap(),
        Some(run.path("result").to_str().unwrap().to_string()),
        BlastBuilder::default(run.database.to_str().unwrap(), Taxon::Bacteria),
        mock,
        &false,
        1,
        ConsensusStrategy::Cautious,
        None,
        OutputFormat::Json,
        None,
    )
}

#[test]
fn run_blast_and_build_consensus_missing_out_parent_is_created() {
    let run = Run::new(&basic_queries());
    let mock = MockBlastn::succeeding_with(&basic_tsv());

    assert!(execute_with_blast_out(&run, "blast-dir/blast", &mock).unwrap());

    assert_eq!(
        read_to_string(run.path("blast-dir/blast.out")).unwrap(),
        basic_tsv()
    );
    assert!(run.path("result.json").is_file());
}

// BUG-R33: every missing ancestor of the BLAST output path is created.
#[test]
fn run_blast_and_build_consensus_missing_out_grandparent_is_created() {
    let run = Run::new(&basic_queries());
    let mock = MockBlastn::succeeding_with(&basic_tsv());

    assert!(execute_with_blast_out(&run, "a/b/blast", &mock).unwrap());

    assert_eq!(
        read_to_string(run.path("a/b/blast.out")).unwrap(),
        basic_tsv()
    );
}

// BUG-R33: a failure to create the output directory is returned as an error
// instead of being ignored (and panicking later on open).
#[test]
fn run_blast_and_build_consensus_uncreatable_out_dir_returns_err() {
    let run = Run::new(&basic_queries());
    let mock = MockBlastn::succeeding_with(&basic_tsv());
    std::fs::write(run.path("not-a-dir"), "").unwrap();

    let err =
        execute_with_blast_out(&run, "not-a-dir/sub/blast", &mock).unwrap_err();

    assert!(
        err.to_string()
            .contains("Could not create blast output directory"),
        "{err}"
    );
}

// BUG-R35 regression: an existing BLAST output without `overwrite` returns an
// error (it used to call `std::process::exit(1)`, which could lose the only
// error message).
#[test]
fn run_blast_and_build_consensus_existing_out_without_overwrite_returns_err() {
    let run = Run::new(&basic_queries());
    write(run.path("blast.out"), "previous").unwrap();
    let mock = MockBlastn::succeeding_with(&basic_tsv());

    let message = error_message(run.execute(&mock));

    assert!(
        message.contains("Could not overwrite existing file"),
        "{message}"
    );
    assert_eq!(mock.call_count(), 0);
    assert_eq!(run.blast_out(), "previous");
}

// BUG-R29 regression (follow-up): an unreadable FASTA input returns an error
// instead of panicking inside `run_parallel_blast`.
#[test]
fn run_blast_and_build_consensus_whitespace_only_header_returns_err() {
    let run = Run::new(&basic_queries());
    write(&run.query, ">   \nACGT\n").unwrap();
    let mock = MockBlastn::succeeding_with(&basic_tsv());

    let message = error_message(run.execute(&mock));

    assert!(
        message.contains("Could not read input sequences"),
        "{message}"
    );
    assert_eq!(mock.call_count(), 0);
}

// BUG-R40: `threads` is the ceiling for the whole run. Chunks used to run on
// the global rayon pool, each `blastn` also getting `-num_threads <threads>`,
// so up to threads x threads host threads were requested.
#[test]
fn run_blast_and_build_consensus_threads_is_a_ceiling_across_parallel_chunks() {
    let run = Run::new(&numbered_headers(400));
    let mock =
        MockBlastn::succeeding_with("").with_delay(Duration::from_millis(50));

    assert!(run.execute_with_threads(&mock, 4).unwrap());

    assert_eq!(mock.call_count(), 8);
    assert!(
        mock.peak_threads() <= 4,
        "peak {} threads (threads per call {:?}, peak concurrent calls {})",
        mock.peak_threads(),
        mock.threads.lock().unwrap(),
        mock.max_in_flight.load(std::sync::atomic::Ordering::SeqCst),
    );
}

// BUG-R40: chunks still run in parallel up to `threads`.
#[test]
fn run_blast_and_build_consensus_runs_chunks_in_parallel_up_to_threads() {
    let run = Run::new(&numbered_headers(400));
    let mock =
        MockBlastn::succeeding_with("").with_delay(Duration::from_millis(50));

    assert!(run.execute_with_threads(&mock, 4).unwrap());

    assert!(mock.max_in_flight.load(std::sync::atomic::Ordering::SeqCst) > 1);
}

// BUG-R40: with fewer chunks than threads, the spare threads go to `blastn`.
#[test]
fn run_blast_and_build_consensus_single_chunk_gets_every_thread() {
    let run = Run::new(&numbered_headers(10));
    let mock = MockBlastn::succeeding_with("");

    assert!(run.execute_with_threads(&mock, 4).unwrap());

    assert_eq!(*mock.threads.lock().unwrap(), vec![4]);
}
