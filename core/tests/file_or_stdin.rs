mod common;

use blul_core::{
    domain::dtos::{
        blast_builder::BlastBuilder,
        blutils_output::BlutilsOutput,
        consensus_result::{ConsensusResult, QueryWithConsensus},
        file_or_stdin::{FileOrStdin, Sequence, StdinError},
        taxon::Taxon,
    },
    use_cases::{write_blutils_output, OutputFormat},
};
use common::fixture;
use serde_json::{json, Value};
use std::{fs, io::ErrorKind, path::Path, str::FromStr};

fn open(path: &Path) -> FileOrStdin {
    FileOrStdin::from_str(path.to_str().unwrap()).unwrap()
}

fn sequences_from(content: &str) -> Result<Vec<Sequence>, StdinError> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.fna");
    fs::write(&path, content).unwrap();
    open(&path).sequence_content()
}

fn pairs(sequences: &[Sequence]) -> Vec<(String, String)> {
    sequences
        .iter()
        .map(|s| (s.header().to_string(), s.sequence().to_string()))
        .collect()
}

fn parse_error_message(err: StdinError) -> String {
    match err {
        StdinError::FromStr(msg) => msg,
        other => panic!("expected StdinError::FromStr, got {other:?}"),
    }
}

fn config_line() -> String {
    serde_json::to_string(&BlastBuilder::default("db.fna", Taxon::Bacteria))
        .unwrap()
}

fn result_line(query: &str) -> String {
    serde_json::to_string(&QueryWithConsensus {
        run_id: None,
        query: query.to_string(),
        taxon: None,
    })
    .unwrap()
}

fn jsonl_from(lines: &[String]) -> Result<BlutilsOutput, StdinError> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.jsonl");
    fs::write(&path, lines.join("\n") + "\n").unwrap();
    open(&path).json_line_content()
}

// ---------------------------------------------------------------------------
// sequence_content
// ---------------------------------------------------------------------------

#[test]
fn sequence_content_multi_line_multi_fasta_concatenates_and_skips_empty_lines()
{
    let sequences = open(&fixture("fasta/multi.fna"))
        .sequence_content()
        .unwrap();

    assert_eq!(
        pairs(&sequences),
        vec![
            ("seq1 first record".to_string(), "ACGTTTGG".to_string()),
            ("seq2".to_string(), "CCCCAAGG".to_string()),
        ]
    );
}

#[test]
fn sequence_content_header_removes_every_gt_character() {
    let sequences = sequences_from(">a>b >c\nACGT\n").unwrap();

    assert_eq!(sequences[0].header(), "ab c");
}

/// Design §3.8 [H] expected `\r` to be preserved; `BufRead::lines` strips
/// the whole `\r\n` terminator instead.
#[test]
fn sequence_content_crlf_line_endings_are_stripped() {
    let sequences =
        sequences_from(">seq1 desc\r\nACGT\r\nTTGG\r\n\r\n>seq2\r\nCC\r\n")
            .unwrap();

    assert_eq!(
        pairs(&sequences),
        vec![
            ("seq1 desc".to_string(), "ACGTTTGG".to_string()),
            ("seq2".to_string(), "CC".to_string()),
        ]
    );
}

#[test]
fn sequence_content_sequence_only_file_returns_empty() {
    let sequences = open(&fixture("fasta/no_header.fna"))
        .sequence_content()
        .unwrap();

    assert!(sequences.is_empty());
}

#[test]
fn sequence_content_sequence_before_header_errors_when_next_header_found() {
    let err = sequences_from("ACGT\n>seq1\nTTGG\n").unwrap_err();

    assert_eq!(
        parse_error_message(err),
        "unexpected sequence without header"
    );
}

#[test]
fn sequence_content_empty_sequence_in_middle_is_kept() {
    let sequences = sequences_from(">empty\n>seq2\nACGT\n").unwrap();

    assert_eq!(
        pairs(&sequences),
        vec![
            ("empty".to_string(), String::new()),
            ("seq2".to_string(), "ACGT".to_string()),
        ]
    );
}

#[test]
fn sequence_content_empty_sequence_at_eof_is_discarded() {
    let sequences = sequences_from(">seq1\nACGT\n>empty\n").unwrap();

    assert_eq!(
        pairs(&sequences),
        vec![("seq1".to_string(), "ACGT".to_string())]
    );
}

#[test]
fn sequence_content_nonexistent_file_returns_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let source = open(&dir.path().join("missing.fna"));

    match source.sequence_content() {
        Err(StdinError::StdIn(err)) => {
            assert_eq!(err.kind(), ErrorKind::NotFound)
        }
        other => panic!("expected StdIn(NotFound), got {other:?}"),
    }
}

#[test]
fn sequence_content_mock_query_file_returns_ten_records() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test/mock/input/query/query.fna");

    let sequences = open(&path).sequence_content().unwrap();

    assert_eq!(sequences.len(), 10);
    assert_eq!(sequences[0].header(), "NR114924.257984.Bac");
    assert!(sequences.iter().all(|s| !s.sequence().is_empty()));
}

// ---------------------------------------------------------------------------
// Sequence accessors
// ---------------------------------------------------------------------------

#[test]
fn blast_header_header_with_description_returns_first_token() {
    let sequences = sequences_from(">NR1.2 some description\nACGT\n").unwrap();

    assert_eq!(sequences[0].blast_header(), "NR1.2");
    assert_eq!(sequences[0].to_fasta(), ">NR1.2 some description\nACGT\n");
}

// BUG-R29: a whitespace-only header is rejected instead of panicking later
// in `blast_header`.
#[test]
fn sequence_content_whitespace_only_header_returns_error() {
    let err = sequences_from(">   \nACGT\n").unwrap_err();

    assert_eq!(parse_error_message(err), "unexpected empty sequence header");
}

// ---------------------------------------------------------------------------
// json_content
// ---------------------------------------------------------------------------

#[test]
fn json_content_valid_blutils_output_parses() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.json");
    let config =
        serde_json::to_value(BlastBuilder::default("db.fna", Taxon::Fungi))
            .unwrap();
    fs::write(
        &path,
        json!({
            "results": [{"runId": null, "query": "q1", "taxon": null}],
            "config": config,
        })
        .to_string(),
    )
    .unwrap();

    let output = open(&path).json_content::<BlutilsOutput>().unwrap();

    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].query, "q1");
    assert_eq!(
        serde_json::to_value(output.config.unwrap()).unwrap(),
        config
    );
}

#[test]
fn json_content_invalid_json_returns_from_str_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.json");
    fs::write(&path, "{not json").unwrap();

    let err = open(&path).json_content::<BlutilsOutput>().unwrap_err();

    assert!(parse_error_message(err)
        .starts_with("unable to parse content as JSON: "));
}

// ---------------------------------------------------------------------------
// json_line_content
// ---------------------------------------------------------------------------

#[test]
fn json_line_content_routes_config_and_results_skipping_empty_lines() {
    let output = jsonl_from(&[
        config_line(),
        String::new(),
        result_line("q1"),
        result_line("q2"),
    ])
    .unwrap();

    let config = serde_json::to_value(output.config.unwrap()).unwrap();
    assert_eq!(config["isConfig"], true);
    assert_eq!(config["subjectReads"], "db.fna");
    assert_eq!(
        output
            .results
            .iter()
            .map(|r| r.query.as_str())
            .collect::<Vec<_>>(),
        vec!["q1", "q2"]
    );
}

#[test]
fn json_line_content_without_config_line_returns_none_config() {
    let output = jsonl_from(&[result_line("q1")]).unwrap();

    assert!(output.config.is_none());
    assert_eq!(output.results.len(), 1);
}

#[test]
fn json_line_content_invalid_line_returns_from_str_error() {
    let err = jsonl_from(&["{oops".to_string()]).unwrap_err();

    assert!(
        parse_error_message(err).starts_with("unable to parse line as JSON: ")
    );
}

// BUG-R09: a `null` config line (older Jsonl outputs) means no config.
#[test]
fn json_line_content_null_first_line_reads_as_no_config() {
    let output = jsonl_from(&["null".to_string(), result_line("q1")]).unwrap();

    assert!(output.config.is_none());
    assert_eq!(output.results.len(), 1);
}

// BUG-R09: lines are routed by the `isConfig` key, not by substring.
#[test]
fn json_line_content_query_containing_is_config_reads_as_result() {
    let output = jsonl_from(&[config_line(), result_line("isConfig")]).unwrap();

    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].query, "isConfig");
}

// ---------------------------------------------------------------------------
// yaml_content
// ---------------------------------------------------------------------------

// Pins the serde_yaml_ng swap: YAML written by `write_blutils_output` is read
// back by `yaml_content` with the same serialization.
#[test]
fn yaml_content_reads_back_write_blutils_output_yaml() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");
    let config = BlastBuilder::default("/db/ref.fna", Taxon::Bacteria);
    let results = vec![
        ConsensusResult::ConsensusFound(QueryWithConsensus {
            run_id: None,
            query: "q1".to_string(),
            taxon: None,
        }),
        ConsensusResult::ConsensusFound(QueryWithConsensus {
            run_id: None,
            query: "q2".to_string(),
            taxon: None,
        }),
    ];

    write_blutils_output(
        results,
        Some(config),
        Some(out.to_str().unwrap().to_string()),
        OutputFormat::Yaml,
    )
    .unwrap();

    let yaml_path = dir.path().join("out.yaml");
    let output = open(&yaml_path).yaml_content::<BlutilsOutput>().unwrap();
    let as_json: Value = serde_json::to_value(&output).unwrap();

    assert_eq!(
        serde_yaml_ng::to_string(&output).unwrap(),
        fs::read_to_string(&yaml_path).unwrap()
    );
    assert_eq!(as_json["config"]["subjectReads"], "ref.fna");
    assert_eq!(as_json["results"][1]["query"], "q2");
}

#[test]
fn yaml_content_invalid_yaml_returns_from_str_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.yaml");
    fs::write(&path, "results: [unclosed").unwrap();

    let err = open(&path).yaml_content::<BlutilsOutput>().unwrap_err();

    assert!(parse_error_message(err)
        .starts_with("unable to parse content as YAML: "));
}
