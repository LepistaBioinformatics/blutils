use blul_core::{
    domain::dtos::{
        blast_builder::BlastBuilder,
        blutils_output::BlutilsOutput,
        consensus_result::{
            ConsensusBean, ConsensusResult, QueryWithConsensus,
            QueryWithoutConsensus,
        },
        linnaean_ranks::LinnaeanRank,
        taxon::Taxon,
        taxonomy_bean::TaxonomyBean,
    },
    use_cases::{write_blutils_output, OutputFormat},
};
use serde_json::Value;
use std::{fs, path::Path};
use uuid::Uuid;

fn found(query: &str, run_id: Option<Uuid>) -> ConsensusResult {
    ConsensusResult::ConsensusFound(QueryWithConsensus {
        run_id,
        query: query.to_string(),
        taxon: Some(TaxonomyBean {
            reached_rank: LinnaeanRank::Genus,
            max_allowed_rank: Some(LinnaeanRank::Species),
            identifier: "561".to_string(),
            perc_identity: 99.5,
            bit_score: 250.0,
            taxonomy: Some("d__2;g__561".to_string()),
            mutated: true,
            single_match: false,
            consensus_beans: Some(vec![ConsensusBean {
                rank: LinnaeanRank::Species,
                identifier: "562".to_string(),
                occurrences: 2,
                taxonomy: Some("d__2;g__561;s__562".to_string()),
                accessions: vec!["NR_1.1".to_string(), "NR_2.1".to_string()],
            }]),
        }),
    })
}

fn not_found(query: &str) -> ConsensusResult {
    ConsensusResult::NoConsensusFound(QueryWithoutConsensus {
        query: query.to_string(),
    })
}

fn config(subject_reads: &str) -> BlastBuilder {
    BlastBuilder::default(subject_reads, Taxon::Bacteria)
}

fn config_run_id(config: &BlastBuilder) -> Value {
    serde_json::to_value(config).unwrap()["runId"].clone()
}

fn write(
    results: Vec<ConsensusResult>,
    config: Option<BlastBuilder>,
    out: &Path,
    format: OutputFormat,
) {
    write_blutils_output(
        results,
        config,
        Some(out.to_str().unwrap().to_string()),
        format,
    )
    .unwrap();
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn jsonl_lines(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

// ---------------------------------------------------------------------------
// Output path handling
// ---------------------------------------------------------------------------

#[test]
fn write_blutils_output_json_forces_json_extension() {
    let dir = tempfile::tempdir().unwrap();

    write(
        vec![],
        None,
        &dir.path().join("out.txt"),
        OutputFormat::Json,
    );

    assert!(dir.path().join("out.json").exists());
    assert!(!dir.path().join("out.txt").exists());
}

#[test]
fn write_blutils_output_jsonl_and_yaml_force_their_extensions() {
    let dir = tempfile::tempdir().unwrap();

    write(vec![], None, &dir.path().join("a.txt"), OutputFormat::Jsonl);
    write(vec![], None, &dir.path().join("b"), OutputFormat::Yaml);

    assert!(dir.path().join("a.jsonl").exists());
    assert!(dir.path().join("b.yaml").exists());
}

#[test]
fn write_blutils_output_existing_txt_input_path_is_left_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let given = dir.path().join("out.txt");
    fs::write(&given, "keep me").unwrap();

    write(vec![], None, &given, OutputFormat::Json);

    assert_eq!(fs::read_to_string(&given).unwrap(), "keep me");
}

/// Only Jsonl reveals the removal: it appends, while Json/Yaml truncate.
#[test]
fn write_blutils_output_existing_output_file_is_removed() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.jsonl");
    fs::write(&out, "stale line\n").unwrap();

    write(
        vec![found("q1", None)],
        Some(config("db.fna")),
        &out,
        OutputFormat::Jsonl,
    );

    let content = fs::read_to_string(&out).unwrap();
    assert!(!content.contains("stale line"));
    assert_eq!(content.lines().count(), 2);
}

#[test]
fn write_blutils_output_missing_parent_directories_are_created() {
    let dir = tempfile::tempdir().unwrap();

    write(
        vec![],
        None,
        &dir.path().join("a/b/out"),
        OutputFormat::Json,
    );

    assert!(dir.path().join("a/b/out.json").exists());
}

// ---------------------------------------------------------------------------
// Result mapping
// ---------------------------------------------------------------------------

#[test]
fn write_blutils_output_no_consensus_found_maps_to_null_taxon_with_run_id() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");
    let config = config("db.fna");
    let run_id = config_run_id(&config);

    write(
        vec![not_found("q1")],
        Some(config),
        &out,
        OutputFormat::Json,
    );

    let json = read_json(&dir.path().join("out.json"));
    assert_eq!(
        json["results"][0],
        serde_json::json!({"runId": run_id, "query": "q1", "taxon": null})
    );
}

#[test]
fn write_blutils_output_with_config_uses_config_run_id_for_all_results() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");
    let config = config("db.fna");
    let run_id = config_run_id(&config);

    write(
        vec![found("q1", None), not_found("q2"), found("q3", None)],
        Some(config),
        &out,
        OutputFormat::Json,
    );

    let json = read_json(&dir.path().join("out.json"));
    assert_eq!(json["config"]["runId"], run_id);
    for result in json["results"].as_array().unwrap() {
        assert_eq!(result["runId"], run_id);
    }
}

#[test]
fn write_blutils_output_without_config_uses_one_new_uuid_for_all_results() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");

    write(
        vec![found("q1", None), not_found("q2")],
        None,
        &out,
        OutputFormat::Json,
    );

    let json = read_json(&dir.path().join("out.json"));
    let first = json["results"][0]["runId"].as_str().unwrap();
    assert!(Uuid::parse_str(first).is_ok());
    assert_eq!(json["results"][1]["runId"], first);
    assert_eq!(json["config"], Value::Null);
}

// BUG-R27: a run id already carried by a `ConsensusFound` is kept.
#[test]
fn write_blutils_output_result_run_id_is_kept_over_config_run_id() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");
    let config = config("db.fna");
    let run_id = config_run_id(&config);
    let own = Uuid::new_v4();

    write(
        vec![found("q1", Some(own))],
        Some(config),
        &out,
        OutputFormat::Json,
    );

    let json = read_json(&dir.path().join("out.json"));
    assert_eq!(json["results"][0]["runId"], own.to_string());
    assert_ne!(json["results"][0]["runId"], run_id);
}

#[test]
fn write_blutils_output_results_sorted_by_query() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");

    write(
        vec![found("q3", None), not_found("q1"), found("q2", None)],
        None,
        &out,
        OutputFormat::Json,
    );

    let json = read_json(&dir.path().join("out.json"));
    let queries: Vec<&str> = json["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["query"].as_str().unwrap())
        .collect();
    assert_eq!(queries, vec!["q1", "q2", "q3"]);
}

#[test]
fn write_blutils_output_consensus_taxon_is_serialized_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");
    let ConsensusResult::ConsensusFound(expected) = found("q1", None) else {
        unreachable!()
    };

    write(vec![found("q1", None)], None, &out, OutputFormat::Json);

    let json = read_json(&dir.path().join("out.json"));
    assert_eq!(
        json["results"][0]["taxon"],
        serde_json::to_value(expected.taxon).unwrap()
    );
}

#[test]
fn write_blutils_output_subject_reads_reduced_to_file_name() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");

    write(
        vec![],
        Some(config("/data/blast/ref.fna")),
        &out,
        OutputFormat::Json,
    );

    let json = read_json(&dir.path().join("out.json"));
    assert_eq!(json["config"]["subjectReads"], "ref.fna");
}

// BUG-R43: a `subject_reads` without a file name used to panic; it is now
// written verbatim.
#[test]
fn write_blutils_output_subject_reads_without_file_name_is_kept_verbatim() {
    for subject_reads in ["", "/", ".."] {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out");

        write(
            vec![],
            Some(config(subject_reads)),
            &out,
            OutputFormat::Json,
        );

        let json = read_json(&dir.path().join("out.json"));
        assert_eq!(json["config"]["subjectReads"], subject_reads);
    }
}

// ---------------------------------------------------------------------------
// Formats
// ---------------------------------------------------------------------------

#[test]
fn write_blutils_output_json_reads_back_as_blutils_output() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");

    write(
        vec![found("q1", None), not_found("q2")],
        Some(config("db.fna")),
        &out,
        OutputFormat::Json,
    );

    let text = fs::read_to_string(dir.path().join("out.json")).unwrap();
    let output: BlutilsOutput = serde_json::from_str(&text).unwrap();
    assert_eq!(output.results.len(), 2);
    assert!(output.config.is_some());
    assert!(text.contains("\n  \"results\""), "expected pretty JSON");
}

#[test]
fn write_blutils_output_jsonl_with_config_writes_config_then_results() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");
    let config = config("db.fna");
    let run_id = config_run_id(&config);

    write(
        vec![found("q2", None), not_found("q1")],
        Some(config),
        &out,
        OutputFormat::Jsonl,
    );

    let lines = jsonl_lines(&dir.path().join("out.jsonl"));
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0]["isConfig"], true);
    assert_eq!(lines[0]["runId"], run_id);
    assert_eq!(lines[1]["query"], "q1");
    assert_eq!(lines[2]["query"], "q2");
}

// BUG-R09: without config no bare `null` line is written.
#[test]
fn write_blutils_output_jsonl_without_config_writes_only_results() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");

    write(vec![not_found("q1")], None, &out, OutputFormat::Jsonl);

    let lines = jsonl_lines(&dir.path().join("out.jsonl"));
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["query"], "q1");
}

#[test]
fn write_blutils_output_yaml_reads_back_as_blutils_output() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");

    write(
        vec![found("q1", None), not_found("q2")],
        Some(config("db.fna")),
        &out,
        OutputFormat::Yaml,
    );

    let output: BlutilsOutput = serde_yaml_ng::from_str(
        &fs::read_to_string(dir.path().join("out.yaml")).unwrap(),
    )
    .unwrap();
    assert_eq!(output.results.len(), 2);
    assert_eq!(output.results[0].query, "q1");
    assert_eq!(
        output.results[0]
            .taxon
            .as_ref()
            .unwrap()
            .consensus_beans
            .as_ref()
            .unwrap()[0]
            .accessions,
        vec!["NR_1.1", "NR_2.1"]
    );
    assert!(output.results[1].taxon.is_none());
}
