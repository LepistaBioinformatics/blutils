mod common;

use common::{blu, core_fixture, parse_blutils_output};
use predicates::prelude::*;
use std::path::Path;

const TSV_HEADER: &str = "run-id\tquery\ttype\trank\tidentifier\tperc-identity\tbit-score\ttaxonomy\tmutated\tsingle-match\toccurrences\taccessions";

fn build_consensus_args(extra: &[&str]) -> Vec<String> {
    let mut args = vec![
        "blastn".to_string(),
        "build-consensus".to_string(),
        core_fixture("blast/basic.tsv").display().to_string(),
        "-t".to_string(),
        core_fixture("taxonomies/basic.blutils.json")
            .display()
            .to_string(),
        "--strategy".to_string(),
        "cautious".to_string(),
    ];
    args.extend(extra.iter().map(|arg| arg.to_string()));
    args
}

fn write_consensus_file(dir: &Path) -> std::path::PathBuf {
    blu(dir)
        .args(build_consensus_args(&[
            "--taxon",
            "bacteria",
            "--blutils-out-file",
            "result",
        ]))
        .assert()
        .success();

    dir.join("result.json")
}

#[test]
fn build_consensus_without_out_file_prints_blutils_output_json() {
    let dir = tempfile::tempdir().unwrap();

    let output = blu(dir.path())
        .args(build_consensus_args(&["--taxon", "bacteria"]))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed = parse_blutils_output(dir.path(), &output);
    let queries: Vec<&str> =
        parsed.results.iter().map(|r| r.query.as_str()).collect();

    assert_eq!(
        queries,
        [
            "q1-single-100",
            "q2-single-98",
            "q3-multi-agree",
            "q4-multi-lca-genus"
        ]
    );
    assert!(parsed.results.iter().all(|r| r.taxon.is_some()));
    // `build-consensus` has no BLAST config to report.
    assert!(parsed.config.is_none());
}

#[test]
fn build_consensus_with_out_file_writes_json_extension_and_empty_stdout() {
    let dir = tempfile::tempdir().unwrap();

    blu(dir.path())
        .args(build_consensus_args(&[
            "--taxon",
            "bacteria",
            "--blutils-out-file",
            "result.txt",
        ]))
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    assert!(dir.path().join("result.json").is_file());
    assert!(!dir.path().join("result.txt").exists());
}

#[test]
fn build_consensus_custom_taxon_without_cutoff_file_panics() {
    let dir = tempfile::tempdir().unwrap();

    blu(dir.path())
        .args(build_consensus_args(&["--taxon", "custom"]))
        .assert()
        .code(101)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "Custom taxon values are required when the custom taxon option is selected.",
        ));
}

#[test]
fn build_tabular_output_file_gets_tsv_extension() {
    let dir = tempfile::tempdir().unwrap();
    let result = write_consensus_file(dir.path());

    blu(dir.path())
        .args(["blastn", "build-tabular"])
        .arg(&result)
        .args(["-o", "out"])
        .assert()
        .success();

    let tsv = std::fs::read_to_string(dir.path().join("out.tsv")).unwrap();
    assert!(tsv.starts_with(TSV_HEADER), "{tsv}");
    assert!(tsv.contains("q1-single-100"), "{tsv}");
    assert!(!dir.path().join("out").exists());
}

#[test]
fn build_tabular_without_positional_reads_blutils_output_from_stdin() {
    let dir = tempfile::tempdir().unwrap();
    let result = write_consensus_file(dir.path());

    let stdout = blu(dir.path())
        .args(["blastn", "build-tabular"])
        .pipe_stdin(&result)
        .unwrap()
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(stdout).unwrap();

    let mut lines = stdout.lines();
    assert_eq!(lines.next(), Some(TSV_HEADER));
    assert!(
        stdout.contains("\tq1-single-100\tconsensus\tspecies\t"),
        "{stdout}"
    );
}

#[test]
fn run_with_consensus_help_shows_query_defaults_to_stdin() {
    let dir = tempfile::tempdir().unwrap();

    blu(dir.path())
        .args(["blastn", "run-with-consensus", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[default: -]"));
}

#[test]
fn invalid_log_level_panics_before_running_the_command() {
    let dir = tempfile::tempdir().unwrap();

    blu(dir.path())
        .args(["--log-level", "[[bad"])
        .args(build_consensus_args(&[
            "--taxon",
            "bacteria",
            "--blutils-out-file",
            "result",
        ]))
        .assert()
        .code(101)
        .stderr(predicate::str::contains(
            "called `Result::unwrap()` on an `Err` value: ParseError",
        ));

    assert!(!dir.path().join("result.json").exists());
}

#[test]
fn log_file_with_ansi_format_replaces_extension_with_log() {
    let dir = tempfile::tempdir().unwrap();

    blu(dir.path())
        .args(["--log-file", "run.txt", "--log-level", "info"])
        .args(build_consensus_args(&[
            "--taxon",
            "bacteria",
            "--blutils-out-file",
            "result",
        ]))
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    let log = std::fs::read_to_string(dir.path().join("run.log")).unwrap();
    assert!(log.contains("Loading Blast results"), "{log}");
    assert!(!dir.path().join("run.txt").exists());
}

#[test]
fn log_file_with_jsonl_format_writes_json_lines_to_jsonl_file() {
    let dir = tempfile::tempdir().unwrap();

    blu(dir.path())
        .args(["--log-file", "run", "--log-format", "jsonl"])
        .args(["--log-level", "info"])
        .args(build_consensus_args(&[
            "--taxon",
            "bacteria",
            "--blutils-out-file",
            "result",
        ]))
        .assert()
        .success();

    let log = std::fs::read_to_string(dir.path().join("run.jsonl")).unwrap();
    let first = log.lines().next().expect("log file must not be empty");
    assert!(first.starts_with('{') && first.ends_with('}'), "{first}");
    assert!(
        log.contains(r#""message":"Loading Blast results""#),
        "{log}"
    );
}

#[test]
fn log_file_existing_file_is_replaced() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("run.log"), "stale line\n").unwrap();

    blu(dir.path())
        .args(["--log-file", "run"])
        .args(build_consensus_args(&[
            "--taxon",
            "bacteria",
            "--blutils-out-file",
            "result",
        ]))
        .assert()
        .success();

    let log = std::fs::read_to_string(dir.path().join("run.log")).unwrap();
    assert!(!log.contains("stale line"), "{log}");
}

fn build_consensus_to_stdout_with_info_logs(dir: &Path) -> String {
    let output = blu(dir)
        .env_remove("RUST_LOG")
        .args(["--log-level", "info", "--log-format", "jsonl"])
        .args(build_consensus_args(&["--taxon", "bacteria"]))
        .assert()
        .success()
        .get_output()
        .clone();

    // stdout stays a clean JSON document: logs go to stderr.
    parse_blutils_output(dir, &output.stdout);

    String::from_utf8(output.stderr).unwrap()
}

// BUG-R20 regression: the dead `RUST_LOG=none` override was removed. Logs
// requested with `--log-level` go to stderr while stdout stays a clean JSON
// document (parsed by the helper above).
#[test]
fn build_consensus_to_stdout_keeps_requested_logs_on_stderr() {
    let dir = tempfile::tempdir().unwrap();

    let stderr = build_consensus_to_stdout_with_info_logs(dir.path());

    assert!(stderr.contains("Loading Blast results"), "{stderr}");
}
