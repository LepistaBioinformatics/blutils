use blul_core::{
    domain::dtos::{
        blast_builder::BlastBuilder,
        consensus_result::{ConsensusResult, QueryWithConsensus},
        file_or_stdin::FileOrStdin,
        taxon::Taxon,
    },
    use_cases::{
        parse_consensus_as_tabular, write_blutils_output, OutputFormat,
    },
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};
use uuid::Uuid;

const HEADER: &str = "run-id\tquery\ttype\trank\tidentifier\tperc-identity\tbit-score\ttaxonomy\tmutated\tsingle-match\toccurrences\taccessions";

const RESULT_RUN_ID: &str = "11111111-1111-1111-1111-111111111111";
const CONFIG_RUN_ID: &str = "22222222-2222-2222-2222-222222222222";

fn taxon(with_beans: bool) -> Value {
    json!({
        "reachedRank": "genus",
        "maxAllowedRank": "species",
        "identifier": "561",
        "percIdentity": 99.5,
        "bitScore": 250.0,
        "taxonomy": "d__2;g__561",
        "mutated": true,
        "singleMatch": false,
        "consensusBeans": if with_beans {
            json!([{
                "rank": "species",
                "identifier": "562",
                "occurrences": 2,
                "taxonomy": "d__2;g__561;s__562",
                "accessions": ["NR_1.1", "NR_2.1"],
            }])
        } else {
            Value::Null
        },
    })
}

fn result(query: &str, run_id: Option<&str>, taxon: Value) -> Value {
    json!({"runId": run_id, "query": query, "taxon": taxon})
}

fn config_with_run_id(run_id: &str) -> Value {
    let mut config =
        serde_json::to_value(BlastBuilder::default("db.fna", Taxon::Bacteria))
            .unwrap();
    config["runId"] = json!(run_id);
    config
}

fn write_input(dir: &Path, results: Vec<Value>, config: Value) -> PathBuf {
    let path = dir.join("in.json");
    fs::write(
        &path,
        json!({"results": results, "config": config}).to_string(),
    )
    .unwrap();
    path
}

fn source(path: &Path) -> FileOrStdin {
    FileOrStdin::from_str(path.to_str().unwrap()).unwrap()
}

/// Runs the use case in file mode and returns the written `.tsv` content.
fn run(dir: &Path, input: &Path, format: OutputFormat) -> String {
    parse_consensus_as_tabular(source(input), Some(dir.join("out")), format)
        .unwrap();
    fs::read_to_string(dir.join("out.tsv")).unwrap()
}

fn main_row(run_id: &str) -> String {
    format!("{run_id}\tq1\tconsensus\tgenus\t561\t99.5\t250\td__2;g__561\ttrue\tfalse\tnull\tnull")
}

fn blast_match_row(run_id: &str) -> String {
    format!("{run_id}\tq1\tblast-match\tspecies\t562\tnull\t250\td__2;g__561;s__562\tnull\tnull\t2\tNR_1.1, NR_2.1")
}

/// Content after the header, valid when nothing follows the main row.
/// Rows are newline-terminated (BUG-R10).
fn single_row(content: &str) -> &str {
    content
        .strip_prefix(&format!("{HEADER}\n"))
        .and_then(|rest| rest.strip_suffix('\n'))
        .expect("header first, one newline-terminated row")
}

// ---------------------------------------------------------------------------
// Row layout (BUG-R10)
// ---------------------------------------------------------------------------

// BUG-R10: every row ends with `\n` and has the 12 header columns, including
// the row of a query without taxon.
#[test]
fn parse_consensus_as_tabular_file_mode_writes_one_row_per_line() {
    let dir = tempfile::tempdir().unwrap();
    let input = write_input(
        dir.path(),
        vec![
            result("q1", Some(RESULT_RUN_ID), taxon(true)),
            result("q2", Some(RESULT_RUN_ID), Value::Null),
        ],
        Value::Null,
    );

    let content = run(dir.path(), &input, OutputFormat::Json);
    let lines: Vec<&str> = content.lines().collect();

    assert!(content.ends_with('\n'));
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0], HEADER);
    assert_eq!(lines[1], main_row(RESULT_RUN_ID));
    assert_eq!(lines[2], blast_match_row(RESULT_RUN_ID));
    for line in lines {
        assert_eq!(line.split('\t').count(), 12, "{line}");
    }
}

#[test]
fn parse_consensus_as_tabular_consensus_row_has_twelve_columns() {
    let dir = tempfile::tempdir().unwrap();
    let input = write_input(
        dir.path(),
        vec![result("q1", Some(RESULT_RUN_ID), taxon(false))],
        Value::Null,
    );

    let content = run(dir.path(), &input, OutputFormat::Json);

    assert_eq!(HEADER.split('\t').count(), 12);
    assert_eq!(single_row(&content), main_row(RESULT_RUN_ID));
    assert_eq!(single_row(&content).split('\t').count(), 12);
}

#[test]
fn parse_consensus_as_tabular_blast_match_row_uses_parent_bit_score_and_null_identity(
) {
    let dir = tempfile::tempdir().unwrap();
    let input = write_input(
        dir.path(),
        vec![result("q1", Some(RESULT_RUN_ID), taxon(true))],
        Value::Null,
    );

    let content = run(dir.path(), &input, OutputFormat::Json);
    // Rows are newline-terminated (BUG-R10).
    let row = content
        .strip_prefix(&format!("{HEADER}\n{}\n", main_row(RESULT_RUN_ID)))
        .and_then(|rest| rest.strip_suffix('\n'))
        .unwrap();
    let columns: Vec<&str> = row.split('\t').collect();

    assert_eq!(columns.len(), 12);
    assert_eq!(columns[2], "blast-match");
    assert_eq!(columns[5], "null");
    assert_eq!(columns[6], "250");
    assert_eq!(columns[10], "2");
    assert_eq!(columns[11], "NR_1.1, NR_2.1");
}

// ---------------------------------------------------------------------------
// run-id resolution
// ---------------------------------------------------------------------------

#[test]
fn parse_consensus_as_tabular_result_run_id_takes_precedence_over_config() {
    let dir = tempfile::tempdir().unwrap();
    let input = write_input(
        dir.path(),
        vec![result("q1", Some(RESULT_RUN_ID), taxon(false))],
        config_with_run_id(CONFIG_RUN_ID),
    );

    let content = run(dir.path(), &input, OutputFormat::Json);

    assert_eq!(single_row(&content), main_row(RESULT_RUN_ID));
}

#[test]
fn parse_consensus_as_tabular_missing_result_run_id_falls_back_to_config() {
    let dir = tempfile::tempdir().unwrap();
    let input = write_input(
        dir.path(),
        vec![result("q1", None, taxon(false))],
        config_with_run_id(CONFIG_RUN_ID),
    );

    let content = run(dir.path(), &input, OutputFormat::Json);

    assert_eq!(single_row(&content), main_row(CONFIG_RUN_ID));
}

#[test]
fn parse_consensus_as_tabular_no_run_id_anywhere_generates_uuid() {
    let dir = tempfile::tempdir().unwrap();
    let input = write_input(
        dir.path(),
        vec![result("q1", None, taxon(false))],
        Value::Null,
    );

    let content = run(dir.path(), &input, OutputFormat::Json);
    let run_id = single_row(&content).split('\t').next().unwrap();

    assert!(Uuid::parse_str(run_id).is_ok(), "{run_id}");
}

// ---------------------------------------------------------------------------
// Output file
// ---------------------------------------------------------------------------

#[test]
fn parse_consensus_as_tabular_output_extension_forced_to_tsv() {
    let dir = tempfile::tempdir().unwrap();
    let input = write_input(dir.path(), vec![], Value::Null);

    parse_consensus_as_tabular(
        source(&input),
        Some(dir.path().join("out.txt")),
        OutputFormat::Json,
    )
    .unwrap();

    // The header is newline-terminated (BUG-R10).
    assert_eq!(
        fs::read_to_string(dir.path().join("out.tsv")).unwrap(),
        format!("{HEADER}\n")
    );
    assert!(!dir.path().join("out.txt").exists());
}

/// Visible because the writer opens the file in append mode.
#[test]
fn parse_consensus_as_tabular_existing_output_file_is_removed() {
    let dir = tempfile::tempdir().unwrap();
    let input = write_input(dir.path(), vec![], Value::Null);
    fs::write(dir.path().join("out.tsv"), "stale\n").unwrap();

    let content = run(dir.path(), &input, OutputFormat::Json);

    // The header is newline-terminated (BUG-R10).
    assert_eq!(content, format!("{HEADER}\n"));
}

// ---------------------------------------------------------------------------
// Input validation (BUG-R11) and formats
// ---------------------------------------------------------------------------

fn write_yaml_input(dir: &Path) -> PathBuf {
    write_blutils_output(
        vec![ConsensusResult::ConsensusFound(QueryWithConsensus {
            run_id: None,
            query: "q1".to_string(),
            taxon: None,
        })],
        None,
        Some(dir.join("in").to_str().unwrap().to_string()),
        OutputFormat::Yaml,
    )
    .unwrap();
    dir.join("in.yaml")
}

// BUG-R11: the existence check uses the actual input path.
#[test]
fn parse_consensus_as_tabular_existing_yaml_without_json_sibling_is_parsed() {
    let dir = tempfile::tempdir().unwrap();
    let input = write_yaml_input(dir.path());

    let content = run(dir.path(), &input, OutputFormat::Yaml);

    assert!(content.starts_with(HEADER));
    assert!(content.contains("q1\tnull"));
}

// BUG-R11: a `.json` sibling no longer stands in for a missing input.
#[test]
fn parse_consensus_as_tabular_missing_input_with_json_sibling_returns_does_not_exist_error(
) {
    let dir = tempfile::tempdir().unwrap();
    write_input(dir.path(), vec![], Value::Null);
    let missing = dir.path().join("in.txt");

    let err = parse_consensus_as_tabular(
        source(&missing),
        Some(dir.path().join("out")),
        OutputFormat::Json,
    )
    .unwrap_err();

    assert!(err.to_string().contains("does not exist"), "{err}");
}

#[test]
fn parse_consensus_as_tabular_missing_input_returns_does_not_exist_error() {
    let dir = tempfile::tempdir().unwrap();

    let err = parse_consensus_as_tabular(
        source(&dir.path().join("in.json")),
        Some(dir.path().join("out")),
        OutputFormat::Json,
    )
    .unwrap_err();

    assert!(err.to_string().contains("does not exist"), "{err}");
}

#[test]
fn parse_consensus_as_tabular_jsonl_with_config_is_parsed() {
    let dir = tempfile::tempdir().unwrap();
    write_blutils_output(
        vec![ConsensusResult::ConsensusFound(QueryWithConsensus {
            run_id: None,
            query: "q1".to_string(),
            taxon: None,
        })],
        Some(BlastBuilder::default("db.fna", Taxon::Bacteria)),
        Some(dir.path().join("in").to_str().unwrap().to_string()),
        OutputFormat::Jsonl,
    )
    .unwrap();

    let content = run(
        dir.path(),
        &dir.path().join("in.jsonl"),
        OutputFormat::Jsonl,
    );

    // No `.json` sibling is needed (BUG-R11); the no-taxon row has 12
    // newline-terminated columns (BUG-R10).
    let row = content.strip_prefix(&format!("{HEADER}\n")).unwrap();
    assert!(row.ends_with('\n'), "{row}");
    let columns: Vec<&str> = row.trim_end_matches('\n').split('\t').collect();
    assert_eq!(columns.len(), 12);
    assert!(Uuid::parse_str(columns[0]).is_ok(), "{}", columns[0]);
    assert_eq!(columns[1], "q1");
    assert!(columns[2..].iter().all(|column| *column == "null"));
}
