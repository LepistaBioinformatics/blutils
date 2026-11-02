//! `ExecuteBlastnProcRepository` resolves `blastn` through the PATH of the
//! test process itself, so every test mutates the process-global `PATH` and
//! must run `#[serial]`, restoring it through `PathGuard`.
#![cfg(unix)]

use blul_core::domain::{
    dtos::{
        blast_builder::{BlastBuilder, Strand},
        taxon::Taxon,
    },
    entities::execute_blastn::{ExecuteBlastn, ExecutionResponse},
};
use blul_proc::execute_blast::ExecuteBlastnProcRepository;
use serial_test::serial;
use std::{ffi::OsString, os::unix::fs::PermissionsExt, path::Path};
use tempfile::TempDir;

struct PathGuard(Option<OsString>);

impl PathGuard {
    fn set(value: &Path) -> Self {
        let previous = std::env::var_os("PATH");
        std::env::set_var("PATH", value);
        PathGuard(previous)
    }
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        match self.0.take() {
            Some(previous) => std::env::set_var("PATH", previous),
            None => std::env::remove_var("PATH"),
        }
    }
}

/// Writes a fake `blastn` with the given shell body into a fresh directory.
/// Only absolute paths are used inside the scripts, since `PATH` will point
/// to this directory alone.
fn fake_blastn(body: &str) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("blastn");

    std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .unwrap();

    dir
}

/// Echoes one argv element per line, then the stdin content.
const ECHO_ARGS_AND_STDIN: &str = r#"for arg in "$@"; do printf '%s\n' "$arg"; done
printf -- '--stdin--\n'
while IFS= read -r line; do printf '%s\n' "$line"; done"#;

fn run(
    config: BlastBuilder,
    threads: usize,
) -> Result<ExecutionResponse, String> {
    ExecuteBlastnProcRepository {}
        .run(">q1\nACGT\n".to_string(), config, threads)
        .map_err(|err| err.to_string())
}

fn success_stdout(response: ExecutionResponse) -> String {
    match response {
        ExecutionResponse::Success(stdout) => stdout,
        ExecutionResponse::Fail(stderr) => panic!("unexpected Fail: {stderr}"),
    }
}

#[test]
#[serial]
fn execute_blastn_default_config_passes_every_flag_as_separate_argv() {
    let bin = fake_blastn(ECHO_ARGS_AND_STDIN);
    let _path = PathGuard::set(bin.path());

    let stdout = success_stdout(
        run(BlastBuilder::default("/data/ref/db", Taxon::Bacteria), 4).unwrap(),
    );
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(
        lines,
        [
            "-db",
            "/data/ref/db",
            "-outfmt",
            "6 qseqid saccver staxid pident length mismatch gapopen qstart qend sstart send evalue bitscore",
            "-max_target_seqs",
            "10",
            "-perc_identity",
            "80",
            "-qcov_hsp_perc",
            "80",
            "-strand",
            "both",
            "-evalue",
            "0.001",
            "-word_size",
            "15",
            "-num_threads",
            "4",
            "--stdin--",
            ">q1",
            "ACGT",
        ]
    );
}

#[test]
#[serial]
fn execute_blastn_custom_config_values_are_forwarded() {
    let bin = fake_blastn(ECHO_ARGS_AND_STDIN);
    let _path = PathGuard::set(bin.path());

    let config = BlastBuilder::default("db", Taxon::Fungi)
        .with_max_target_seqs(5)
        .with_perc_identity(97)
        .with_query_cov(90)
        .with_strand(Strand::Minus)
        .with_e_value(1e-5)
        .with_word_size(11);

    let stdout = success_stdout(run(config, 1).unwrap());
    let lines: Vec<&str> = stdout.lines().collect();
    let value_of = |flag: &str| {
        let index = lines.iter().position(|line| *line == flag).unwrap();
        lines[index + 1]
    };

    assert_eq!(value_of("-max_target_seqs"), "5");
    assert_eq!(value_of("-perc_identity"), "97");
    assert_eq!(value_of("-qcov_hsp_perc"), "90");
    assert_eq!(value_of("-strand"), "minus");
    // `f32` Display formatting, not scientific notation.
    assert_eq!(value_of("-evalue"), "0.00001");
    assert_eq!(value_of("-word_size"), "11");
    assert_eq!(value_of("-num_threads"), "1");
}

#[test]
#[serial]
fn execute_blastn_zero_exit_returns_success_with_stdout() {
    let bin = fake_blastn("printf 'q1\\tACC1.1\\n'; printf 'warning\\n' >&2");
    let _path = PathGuard::set(bin.path());

    let response =
        run(BlastBuilder::default("db", Taxon::Bacteria), 1).unwrap();

    assert_eq!(success_stdout(response), "q1\tACC1.1\n");
}

#[test]
#[serial]
fn execute_blastn_non_zero_exit_returns_fail_with_stderr() {
    let bin = fake_blastn(
        "printf 'partial\\n'; printf 'BLAST Database error\\n' >&2; exit 2",
    );
    let _path = PathGuard::set(bin.path());

    let response =
        run(BlastBuilder::default("db", Taxon::Bacteria), 1).unwrap();

    match response {
        ExecutionResponse::Fail(stderr) => {
            assert_eq!(stderr, "BLAST Database error\n")
        }
        ExecutionResponse::Success(stdout) => {
            panic!("unexpected Success: {stdout}")
        }
    }
}

#[test]
#[serial]
fn execute_blastn_missing_binary_returns_err() {
    // An existing but empty directory: an empty/unset PATH would make execvp
    // fall back to a default search path.
    let empty = tempfile::tempdir().unwrap();
    let _path = PathGuard::set(empty.path());

    let err = run(BlastBuilder::default("db", Taxon::Bacteria), 1).unwrap_err();

    assert!(
        err.contains("Unexpected error detected on execute blast"),
        "{err}"
    );
}
