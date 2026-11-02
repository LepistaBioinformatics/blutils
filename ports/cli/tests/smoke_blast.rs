//! Smoke tests against a real NCBI BLAST+ installation (`makeblastdb`,
//! `blastn`, `blastdbcmd`). Not run by default:
//! `cargo test -p blutils-cli --test smoke_blast -- --ignored`.
#![cfg(unix)]

mod common;

use blul_core::domain::{
    dtos::{blast_builder::BlastBuilder, taxon::Taxon},
    entities::execute_blastn::{ExecuteBlastn, ExecutionResponse},
};
use blul_proc::execute_blast::ExecuteBlastnProcRepository;
use common::{blu, manifest_dir, parse_blutils_output};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn mock_input(relative: &str) -> PathBuf {
    manifest_dir().join("../../test/mock/input").join(relative)
}

/// `(seqid, numeric lineage)` pairs from the mock taxonomy TSV, deduplicated.
fn mock_taxonomies() -> BTreeMap<String, String> {
    fs::read_to_string(mock_input("ref_databases/mock-16S_taxonomies.tsv"))
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let (seqid, lineage) = line.split_once('\t').unwrap();
            (seqid.trim().to_string(), lineage.trim().to_string())
        })
        .collect()
}

/// Taxid of the deepest rank in the lineage; some mock lineages stop above
/// species (e.g. at `f__` or `o__`).
fn leaf_taxid(lineage: &str) -> &str {
    lineage.rsplit_once("__").unwrap().1
}

/// Builds `<dir>/db` with `makeblastdb` (taxids mapped from the mock TSV) and
/// touches `taxdb.btd`, which `validate_blast_database` requires but
/// `makeblastdb` does not create.
fn make_blast_database(dir: &Path) -> PathBuf {
    let taxid_map = dir.join("taxid_map.txt");
    let map: String = mock_taxonomies()
        .iter()
        .map(|(seqid, lineage)| format!("{seqid} {}\n", leaf_taxid(lineage)))
        .collect();
    fs::write(&taxid_map, map).unwrap();

    let db = dir.join("db");
    let status = Command::new("makeblastdb")
        .arg("-in")
        .arg(mock_input("ref_databases/mock-16S.fna"))
        .args(["-dbtype", "nucl", "-parse_seqids", "-taxid_map"])
        .arg(&taxid_map)
        .arg("-out")
        .arg(&db)
        .status()
        .expect("makeblastdb must be installed");
    assert!(status.success());

    fs::write(dir.join("taxdb.btd"), b"").unwrap();
    db
}

/// Blutils taxonomy database (JSON) built from the mock TSV, using the
/// numeric lineage for both text and numeric fields.
fn write_taxonomies_json(dir: &Path) -> PathBuf {
    let mut by_taxid = BTreeMap::<String, (String, Vec<String>)>::new();

    for (seqid, lineage) in mock_taxonomies() {
        by_taxid
            .entry(leaf_taxid(&lineage).to_string())
            .or_insert_with(|| (lineage.clone(), Vec::new()))
            .1
            .push(seqid);
    }

    let records: Vec<String> = by_taxid
        .iter()
        .map(|(taxid, (lineage, seqids))| {
            let accessions: Vec<String> = seqids
                .iter()
                .enumerate()
                .map(|(oid, seqid)| {
                    format!(r#"{{"accession":"{seqid}","oid":"{oid}"}}"#)
                })
                .collect();

            format!(
                r#"{{"taxid":{taxid},"rank":"species","numericLineage":"{lineage}","textLineage":"{lineage}","accessions":[{}]}}"#,
                accessions.join(",")
            )
        })
        .collect();

    let path = dir.join("taxonomies.json");
    fs::write(
        &path,
        format!(
            r#"{{"blutilsVersion":"{}","ignoreTaxids":null,"replaceRank":null,"dropNonLinnaeanTaxonomies":null,"sourceDatabase":"mock-16S","taxonomies":[{}]}}"#,
            env!("CARGO_PKG_VERSION"),
            records.join(",")
        ),
    )
    .unwrap();

    path
}

#[test]
#[ignore = "requires ncbi-blast+"]
fn smoke_check_linux_with_real_blast_succeeds() {
    let dir = tempfile::tempdir().unwrap();

    blu(dir.path()).args(["check", "linux"]).assert().success();
}

#[test]
#[ignore = "requires ncbi-blast+"]
fn smoke_execute_blastn_proc_repository_returns_tabular_hits() {
    let dir = tempfile::tempdir().unwrap();
    let db = make_blast_database(dir.path());

    let response = ExecuteBlastnProcRepository {}
        .run(
            fs::read_to_string(mock_input("query/query.fna")).unwrap(),
            BlastBuilder::default(db.to_str().unwrap(), Taxon::Bacteria),
            1,
        )
        .unwrap();

    let stdout = match response {
        ExecutionResponse::Success(stdout) => stdout,
        ExecutionResponse::Fail(stderr) => panic!("blastn failed: {stderr}"),
    };

    assert!(!stdout.is_empty());
    assert!(stdout.lines().all(|line| line.split('\t').count() == 13));
}

#[test]
#[ignore = "requires ncbi-blast+"]
fn smoke_run_with_consensus_with_real_blast_prints_one_result_per_query() {
    let dir = tempfile::tempdir().unwrap();
    let db = make_blast_database(dir.path());
    let taxonomies = write_taxonomies_json(dir.path());
    let query = mock_input("query/query.fna");

    let stdout = blu(dir.path())
        .args(["blastn", "run-with-consensus"])
        .arg(&query)
        .arg("-d")
        .arg(&db)
        .arg("-t")
        .arg(&taxonomies)
        .args(["--blast-out-file", "blast/run"])
        .args(["--taxon", "bacteria", "--strategy", "cautious"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed = parse_blutils_output(dir.path(), &stdout);
    let queries = fs::read_to_string(&query)
        .unwrap()
        .lines()
        .filter(|line| line.starts_with('>'))
        .count();

    assert_eq!(parsed.results.len(), queries);
    assert!(parsed.config.is_some());
    assert!(parsed.results.iter().any(|result| result.taxon.is_some()));
    assert!(!fs::read_to_string(dir.path().join("blast/run.out"))
        .unwrap()
        .is_empty());
}
