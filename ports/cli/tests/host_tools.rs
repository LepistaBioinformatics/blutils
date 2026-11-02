//! Use cases that shell out without a seam (`which`, `blastn`, `blastdbcmd`),
//! driven through the `blu` binary with fake executables on the CHILD's `PATH`
//! only (`assert_cmd .env`), so the test process environment is never mutated.
#![cfg(unix)]

mod common;

use common::{
    blu, cli_fixture, core_fixture, fake_blast_database, fake_path,
    parse_blutils_output,
};
use predicates::prelude::*;
use std::{fs, path::Path};

const MISSING_DEPENDENCIES: &str = "Missing dependencies detected. Please install them before running the command again.";

// ? ---------------------------------------------------------------------------
// ? check_host_requirements (`blu check linux`)
// ? ---------------------------------------------------------------------------

#[test]
fn check_linux_with_blastn_and_blastdbcmd_on_path_succeeds() {
    let dir = tempfile::tempdir().unwrap();

    blu(dir.path())
        .env("PATH", fake_path("all"))
        .args(["check", "linux"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn check_linux_with_blastdbcmd_missing_fails_with_missing_dependencies() {
    let dir = tempfile::tempdir().unwrap();

    blu(dir.path())
        .env("PATH", fake_path("no_blastdbcmd"))
        .args(["check", "linux"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains(MISSING_DEPENDENCIES));
}

#[test]
fn check_linux_without_which_fails_with_unexpected_error() {
    let dir = tempfile::tempdir().unwrap();
    // An existing but empty directory: an empty/unset PATH would make execvp
    // fall back to a default search path containing the real `which`.
    let empty = tempfile::tempdir().unwrap();

    blu(dir.path())
        .env("PATH", empty.path())
        .args(["check", "linux"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains(
            "Unexpected error detected on check host system",
        ));
}

#[test]
fn build_db_kraken2_with_missing_host_dependency_fails_before_touching_output()
{
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");
    fs::create_dir(&out).unwrap();
    fs::write(out.join("sentinel"), "keep").unwrap();

    blu(dir.path())
        .env("PATH", fake_path("no_blastdbcmd"))
        .args(["build-db", "kraken2", "db", "-o"])
        .arg(&out)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains(MISSING_DEPENDENCIES));

    assert!(out.join("sentinel").is_file());
}

// ? ---------------------------------------------------------------------------
// ? run-with-consensus (fake `blastn`)
// ? ---------------------------------------------------------------------------

fn run_with_consensus(dir: &Path, extra: &[&str]) -> assert_cmd::Command {
    let db = fake_blast_database(dir);
    let mut cmd = blu(dir);

    cmd.env("PATH", fake_path("all"))
        .env("FAKE_BLASTN_STDOUT_FILE", core_fixture("blast/basic.tsv"))
        .args(["blastn", "run-with-consensus"])
        .args(extra)
        .arg("-d")
        .arg(&db)
        .arg("-t")
        .arg(core_fixture("taxonomies/basic.blutils.json"))
        .args(["--blast-out-file", "blast/run"])
        .args(["--taxon", "bacteria", "--strategy", "cautious"]);

    cmd
}

#[test]
fn run_with_consensus_with_fake_blastn_prints_results_with_config() {
    let dir = tempfile::tempdir().unwrap();
    let query = cli_fixture("query.fna");

    let stdout = run_with_consensus(dir.path(), &[query.to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed = parse_blutils_output(dir.path(), &stdout);
    let queries: Vec<&str> =
        parsed.results.iter().map(|r| r.query.as_str()).collect();

    assert_eq!(
        queries,
        [
            "q1-single-100",
            "q2-single-98",
            "q3-multi-agree",
            "q4-multi-lca-genus",
            "q9-no-hits"
        ]
    );
    // A query sent to BLAST without hits is reported without taxon.
    assert!(parsed.results[4].taxon.is_none());
    assert!(parsed.results[..4].iter().all(|r| r.taxon.is_some()));

    let config = parsed
        .config
        .expect("run-with-consensus reports its config");
    assert_eq!(config.out_format.split_whitespace().next(), Some("6"));
    assert_eq!(config.max_target_seqs, 10);

    // The raw BLAST output is persisted with the `.out` extension (created dir).
    assert_eq!(
        fs::read_to_string(dir.path().join("blast/run.out")).unwrap(),
        fs::read_to_string(core_fixture("blast/basic.tsv")).unwrap()
    );
}

#[test]
fn run_with_consensus_without_query_reads_fasta_from_stdin() {
    let dir = tempfile::tempdir().unwrap();

    let stdout = run_with_consensus(dir.path(), &[])
        .pipe_stdin(cli_fixture("query.fna"))
        .unwrap()
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed = parse_blutils_output(dir.path(), &stdout);
    assert_eq!(parsed.results.len(), 5);
}

// BUG-R35 regression: the "existing output" error is returned instead of
// `std::process::exit(1)`, so the message reliably reaches stderr.
#[test]
fn run_with_consensus_existing_out_file_without_force_reports_error() {
    let dir = tempfile::tempdir().unwrap();
    let query = cli_fixture("query.fna");
    fs::create_dir(dir.path().join("blast")).unwrap();
    fs::write(dir.path().join("blast/run.out"), "previous").unwrap();

    for _ in 0..5 {
        run_with_consensus(dir.path(), &[query.to_str().unwrap()])
            .assert()
            .failure()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains(
                "Could not overwrite existing file",
            ));
    }

    assert_eq!(
        fs::read_to_string(dir.path().join("blast/run.out")).unwrap(),
        "previous"
    );
}

#[test]
fn run_with_consensus_existing_out_file_with_force_is_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let query = cli_fixture("query.fna");
    fs::create_dir(dir.path().join("blast")).unwrap();
    fs::write(dir.path().join("blast/run.out"), "previous").unwrap();

    run_with_consensus(dir.path(), &[query.to_str().unwrap(), "-f"])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(dir.path().join("blast/run.out")).unwrap(),
        fs::read_to_string(core_fixture("blast/basic.tsv")).unwrap()
    );
}

#[test]
fn run_with_consensus_custom_taxon_without_cutoff_file_fails_after_host_check()
{
    let dir = tempfile::tempdir().unwrap();
    let db = fake_blast_database(dir.path());

    blu(dir.path())
        .env("PATH", fake_path("all"))
        .args(["blastn", "run-with-consensus"])
        .arg(cli_fixture("query.fna"))
        .arg("-d")
        .arg(&db)
        .arg("-t")
        .arg(core_fixture("taxonomies/basic.blutils.json"))
        .args(["--blast-out-file", "blast/run"])
        .args(["--taxon", "custom", "--strategy", "cautious"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains(
            "Custom taxon values are required when the custom taxon option is selected (use --custom-taxon-cutoff-file).",
        ));

    assert!(!dir.path().join("blast").exists());
}

// ? ---------------------------------------------------------------------------
// ? build-db qiime2 (fake `blastdbcmd`)
// ? ---------------------------------------------------------------------------

const QIIME_TSV_HEAD: &str = "Feature ID\tTaxon\n\
101-0-ACC101.1\td__bacteria;p__bacillota;c__bacilli;o__bacillales;f__bacillaceae;g__bacillus;s__bacillus-subtilis\n";

#[test]
fn build_db_qiime2_writes_taxonomies_tsv_before_validating_the_database() {
    let dir = tempfile::tempdir().unwrap();

    blu(dir.path())
        .env("PATH", fake_path("all"))
        .args(["build-db", "qiime2"])
        .arg(core_fixture("taxonomies/basic.blutils.json"))
        .args(["tax", "missing-db", "seqs"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains("Blast database not found"));

    let tsv = fs::read_to_string(dir.path().join("tax.tsv")).unwrap();
    assert!(tsv.starts_with(QIIME_TSV_HEAD), "{tsv}");
    assert!(!dir.path().join("seqs.fna").exists());
}

#[test]
fn build_db_qiime2_use_taxid_writes_numeric_lineage() {
    let dir = tempfile::tempdir().unwrap();

    blu(dir.path())
        .env("PATH", fake_path("all"))
        .args(["build-db", "qiime2"])
        .arg(core_fixture("taxonomies/basic.blutils.json"))
        .args(["tax", "missing-db", "seqs", "-u"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("panicked").not());

    let tsv = fs::read_to_string(dir.path().join("tax.tsv")).unwrap();
    assert!(
        tsv.starts_with(
            "Feature ID\tTaxon\n\
             101-0-ACC101.1\td__2;p__1239;c__91061;o__1385;f__186817;g__1386;s__1423\n"
        ),
        "{tsv}"
    );
}

#[test]
fn build_db_qiime2_with_fake_blastdbcmd_writes_fna_records() {
    let dir = tempfile::tempdir().unwrap();
    let db = fake_blast_database(dir.path());

    blu(dir.path())
        .env("PATH", fake_path("all"))
        .env(
            "FAKE_BLASTDBCMD_STDOUT",
            "ACC1.1  101  0  acgt\nACC2.1  102  1  GGCC\n",
        )
        .args(["build-db", "qiime2"])
        .arg(core_fixture("taxonomies/basic.blutils.json"))
        .args(["tax.txt"])
        .arg(&db)
        .args(["seqs.txt"])
        .assert()
        .success();

    // Sequences are written verbatim (no upper-casing, no wrapping).
    assert_eq!(
        fs::read_to_string(dir.path().join("seqs.fna")).unwrap(),
        ">101-0-ACC1.1\nacgt\n>102-1-ACC2.1\nGGCC\n"
    );
    assert!(dir.path().join("tax.tsv").is_file());
    assert!(!dir.path().join("tax.txt").exists());
}

#[test]
fn build_db_qiime2_blastdbcmd_line_with_less_than_four_fields_fails_with_error()
{
    let dir = tempfile::tempdir().unwrap();
    let db = fake_blast_database(dir.path());

    blu(dir.path())
        .env("PATH", fake_path("all"))
        .env("FAKE_BLASTDBCMD_STDOUT", "ACC1.1  101  acgt\n")
        .args(["build-db", "qiime2"])
        .arg(core_fixture("taxonomies/basic.blutils.json"))
        .args(["tax"])
        .arg(&db)
        .args(["seqs"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains(
            "Invalid line detected on blastdbcmd response",
        ));
}

// BUG-R36 regression: missing output parent directories are created.
#[test]
fn build_db_qiime2_missing_output_parent_directory_is_created() {
    let dir = tempfile::tempdir().unwrap();

    blu(dir.path())
        .env("PATH", fake_path("all"))
        .args(["build-db", "qiime2"])
        .arg(core_fixture("taxonomies/basic.blutils.json"))
        .args(["missing/tax", "missing-db", "seqs"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains("Blast database not found"));

    assert!(dir.path().join("missing/tax.tsv").is_file());
}

// ? ---------------------------------------------------------------------------
// ? build-db kraken2 (fake `blastdbcmd`)
// ? ---------------------------------------------------------------------------

fn kraken2_with_missing_database(dir: &Path, out: &Path) {
    blu(dir)
        .env("PATH", fake_path("all"))
        .args(["build-db", "kraken2", "missing-db", "-o"])
        .arg(out)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains("Blast database not found"));
}

// BUG-R12 regression: the database is validated before the output directory
// is touched, so a failing run keeps previous content.
#[test]
fn build_db_kraken2_missing_database_keeps_output_directory() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");
    fs::create_dir(&out).unwrap();
    fs::write(out.join("sentinel"), "keep").unwrap();

    kraken2_with_missing_database(dir.path(), &out);

    assert!(out.join("sentinel").is_file());
}

// BUG-R12 regression: a failing run leaves an existing output file untouched.
#[test]
fn build_db_kraken2_missing_database_keeps_output_file() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");
    fs::write(&out, "not a directory").unwrap();

    kraken2_with_missing_database(dir.path(), &out);

    assert_eq!(fs::read_to_string(&out).unwrap(), "not a directory");
}

#[test]
fn build_db_kraken2_with_fake_blastdbcmd_writes_library_and_prelim_map() {
    let dir = tempfile::tempdir().unwrap();
    let db = fake_blast_database(dir.path());
    let out = dir.path().join("kraken");
    let long = "a".repeat(170);

    blu(dir.path())
        .env("PATH", fake_path("all"))
        .env(
            "FAKE_BLASTDBCMD_STDOUT",
            format!("ACC1.1  101  acgt\nACC2.1  102  {long}\n"),
        )
        .args(["build-db", "kraken2"])
        .arg(&db)
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let a80 = "A".repeat(80);
    assert_eq!(
        fs::read_to_string(out.join("library.fna")).unwrap(),
        format!(
            ">kraken:taxid|101|ACC1.1\nACGT\n\
             >kraken:taxid|102|ACC2.1\n{a80}\n{a80}\n{}\n",
            "A".repeat(10)
        )
    );
    assert_eq!(
        fs::read_to_string(out.join("prelim_map.txt")).unwrap(),
        "TAXID\tkraken:taxid|101|ACC1.1\t101\n\
         TAXID\tkraken:taxid|102|ACC2.1\t102\n"
    );
}

#[test]
fn build_db_kraken2_blastdbcmd_line_with_less_than_three_fields_fails_with_error(
) {
    let dir = tempfile::tempdir().unwrap();
    let db = fake_blast_database(dir.path());

    blu(dir.path())
        .env("PATH", fake_path("all"))
        .env("FAKE_BLASTDBCMD_STDOUT", "ACC1.1  101\n")
        .args(["build-db", "kraken2"])
        .arg(&db)
        .args(["-o", "kraken"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains(
            "Invalid line detected on blastdbcmd response",
        ));
}

// ? ---------------------------------------------------------------------------
// ? build-db blu: build_accessions_map (fake `blastdbcmd`)
// ? ---------------------------------------------------------------------------

fn build_db_blu(
    dir: &Path,
    blastdbcmd_stdout: &str,
) -> assert_cmd::assert::Assert {
    let db = fake_blast_database(dir);

    blu(dir)
        .env("PATH", fake_path("all"))
        .env("FAKE_BLASTDBCMD_STDOUT", blastdbcmd_stdout)
        .args(["build-db", "blu"])
        .arg(&db)
        .args(["missing-taxdump", "out.json"])
        .assert()
}

#[test]
fn build_db_blu_valid_accessions_reach_the_taxdump_validation() {
    let dir = tempfile::tempdir().unwrap();

    build_db_blu(dir.path(), "ACC1.1  101  0\nACC2.1  101  1\n")
        .code(1)
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains("Invalid taxdump directory path"));
}

#[test]
fn build_db_blu_non_integer_taxid_fails_with_error() {
    let dir = tempfile::tempdir().unwrap();

    build_db_blu(dir.path(), "ACC1.1  abc  0\n")
        .code(1)
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains(
            "Invalid taxid detected on blastdbcmd response for ACC1.1: \"abc\"",
        ));
}

#[test]
fn build_db_blu_line_with_less_than_three_fields_fails_with_error() {
    let dir = tempfile::tempdir().unwrap();

    build_db_blu(dir.path(), "ACC1.1  101\n")
        .code(1)
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains(
            "Invalid line detected on blastdbcmd response",
        ));
}
