//! Data contract between the engine (`BlutilsOutput`) and the viewer
//! (`blutils-ui/src/types/BlutilsResult.ts`).
//!
//! Builds a golden `blutils.contract.json` from dedicated fixtures under
//! `tests/fixtures/contract/` and compares it with the committed copy.
//!
//! To regenerate after an intended change of the serialized output:
//!
//! ```bash
//! UPDATE_GOLDEN=1 cargo test -p blutils-core --test contract
//! cp core/tests/fixtures/contract/blutils.contract.json \
//!    ../blutils-ui/src/__fixtures__/blutils.contract.json
//! ```
//!
//! The UI copy must stay byte-for-byte identical (checked by the monorepo
//! `contract.yml` workflow).

mod common;

use blul_core::{
    domain::dtos::{
        blast_builder::BlastBuilder, blutils_output::BlutilsOutput,
        consensus_strategy::ConsensusStrategy,
        parallel_blast_output::ParallelBlastOutput, taxon::Taxon,
    },
    use_cases::{
        build_consensus_identities, write_blutils_output, OutputFormat,
    },
};
use common::fixture;
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::PathBuf};

const GOLDEN: &str = "contract/blutils.contract.json";
const NORMALIZED_RUN_ID: &str = "00000000-0000-0000-0000-000000000000";
const NORMALIZED_BLUTILS_VERSION: &str = "0.0.0-contract";

const BLUTILS_OUTPUT_KEYS: &[&str] = &["results", "config"];
const QUERY_WITH_CONSENSUS_KEYS: &[&str] = &["runId", "query", "taxon"];
const TAXONOMY_BEAN_KEYS: &[&str] = &[
    "reachedRank",
    "maxAllowedRank",
    "identifier",
    "percIdentity",
    "bitScore",
    "taxonomy",
    "mutated",
    "singleMatch",
    "consensusBeans",
];
const CONSENSUS_BEAN_KEYS: &[&str] = &[
    "rank",
    "identifier",
    "occurrences",
    "taxonomy",
    "accessions",
];
const BLAST_BUILDER_KEYS: &[&str] = &[
    "isConfig",
    "runId",
    "blutilsVersion",
    "subjectReads",
    "taxon",
    "outFormat",
    "maxTargetSeqs",
    "percIdentity",
    "queryCov",
    "strand",
    "eValue",
    "wordSize",
];

fn golden_path() -> PathBuf {
    fixture(GOLDEN)
}

fn headers() -> Vec<String> {
    fs::read_to_string(fixture("contract/headers.txt"))
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(String::from)
        .collect()
}

/// Replaces every non-null `runId` (results and config) with a fixed value.
fn normalize_run_ids(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if key == "runId" && !child.is_null() {
                    *child = Value::String(NORMALIZED_RUN_ID.to_string());
                } else {
                    normalize_run_ids(child);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(normalize_run_ids),
        _ => {}
    }
}

/// Runs the engine over the contract fixtures and returns the normalized
/// output as a `Value`.
fn generate() -> Value {
    let dir = tempfile::tempdir().unwrap();

    let results = build_consensus_identities(
        ParallelBlastOutput {
            output_file: fixture("contract/blast.tsv"),
            headers: Some(headers()),
        },
        &fixture("contract/taxonomies.blutils.json"),
        Taxon::Bacteria,
        ConsensusStrategy::Cautious,
        None,
        None,
    )
    .unwrap();

    let out = dir.path().join("blutils.contract");
    write_blutils_output(
        results,
        Some(BlastBuilder::default(
            "/data/contract-16S.fna",
            Taxon::Bacteria,
        )),
        Some(out.to_str().unwrap().to_string()),
        OutputFormat::Json,
    )
    .unwrap();

    let mut value: Value = serde_json::from_str(
        &fs::read_to_string(out.with_extension("json")).unwrap(),
    )
    .unwrap();

    normalize_run_ids(&mut value);
    value["config"]["blutilsVersion"] =
        Value::String(NORMALIZED_BLUTILS_VERSION.to_string());
    value["results"]
        .as_array_mut()
        .unwrap()
        .sort_by_key(|record| record["query"].as_str().unwrap().to_string());

    value
}

/// Serializes through the struct so the key order follows the field
/// declaration order, independently of serde_json's `preserve_order` feature.
fn render(value: &Value) -> String {
    let output: BlutilsOutput = serde_json::from_value(value.clone()).unwrap();
    format!("{}\n", serde_json::to_string_pretty(&output).unwrap())
}

fn mismatch_report(expected: &str, actual: &str) -> String {
    let expected_lines = expected.lines().collect::<Vec<_>>();
    let actual_lines = actual.lines().collect::<Vec<_>>();
    let mut report = Vec::new();

    for index in 0..expected_lines.len().max(actual_lines.len()) {
        let left = expected_lines.get(index).copied();
        let right = actual_lines.get(index).copied();
        if left != right {
            report.push(format!(
                "line {}:\n  - golden:    {}\n  + generated: {}",
                index + 1,
                left.unwrap_or("<missing>"),
                right.unwrap_or("<missing>"),
            ));
        }
        if report.len() == 20 {
            report.push("... (truncated)".to_string());
            break;
        }
    }

    report.join("\n")
}

fn key_set(value: &Value) -> BTreeSet<String> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("expected an object, got {value}"))
        .keys()
        .cloned()
        .collect()
}

fn assert_keys(value: &Value, expected: &[&str], what: &str) {
    let expected = expected
        .iter()
        .map(|key| key.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(key_set(value), expected, "key set of {what} changed");
}

fn results(value: &Value) -> &Vec<Value> {
    value["results"].as_array().unwrap()
}

fn taxa(value: &Value) -> Vec<&Value> {
    results(value)
        .iter()
        .map(|record| &record["taxon"])
        .filter(|taxon| !taxon.is_null())
        .collect()
}

#[test]
fn contract_golden_matches_generated_output() {
    let actual = render(&generate());
    let path = golden_path();

    if std::env::var("UPDATE_GOLDEN").as_deref() == Ok("1") {
        fs::write(&path, &actual).unwrap();
        return;
    }

    let expected = fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!(
            "cannot read golden {}: {err}\n\
             generate it with: UPDATE_GOLDEN=1 cargo test -p blutils-core --test contract",
            path.display()
        )
    });

    if expected != actual {
        panic!(
            "golden {} differs from the generated output:\n{}\n\n\
             If the change is intended, regenerate it with\n  \
             UPDATE_GOLDEN=1 cargo test -p blutils-core --test contract\n\
             and copy it to blutils-ui/src/__fixtures__/blutils.contract.json \
             (update blutils-ui/src/types/BlutilsResult.ts accordingly).",
            path.display(),
            mismatch_report(&expected, &actual),
        );
    }
}

#[test]
fn contract_golden_deserializes_into_blutils_output() {
    let golden = fs::read_to_string(golden_path()).unwrap();
    let output: BlutilsOutput = serde_json::from_str(&golden).unwrap();

    assert!(output.config.is_some());
    assert_eq!(output.results.len(), headers().len());
}

#[test]
fn contract_output_has_exact_key_sets() {
    let value = generate();

    assert_keys(&value, BLUTILS_OUTPUT_KEYS, "BlutilsOutput");
    assert_keys(
        &value["config"],
        BLAST_BUILDER_KEYS,
        "BlastBuilder (config)",
    );

    for record in results(&value) {
        assert_keys(record, QUERY_WITH_CONSENSUS_KEYS, "QueryWithConsensus");
    }

    let taxa = taxa(&value);
    assert!(!taxa.is_empty());
    for taxon in &taxa {
        assert_keys(taxon, TAXONOMY_BEAN_KEYS, "TaxonomyBean");
    }

    let beans = taxa
        .iter()
        .filter_map(|taxon| taxon["consensusBeans"].as_array())
        .flatten()
        .collect::<Vec<_>>();
    assert!(!beans.is_empty());
    for bean in beans {
        assert_keys(bean, CONSENSUS_BEAN_KEYS, "ConsensusBean");
    }
}

#[test]
fn contract_output_covers_required_cases() {
    let value = generate();
    let records = results(&value);
    let taxa = taxa(&value);

    assert!(
        records.iter().any(|record| record["taxon"].is_null()),
        "missing a query without consensus (taxon: null)"
    );
    assert!(
        taxa.iter().any(|taxon| taxon["singleMatch"] == true
            && taxon["percIdentity"] == 100.0
            && taxon["reachedRank"] == "strain"),
        "missing a single 100% hit reaching a non-Linnaean rank"
    );
    assert!(
        taxa.iter().any(|taxon| taxon["singleMatch"] == false
            && !taxon["maxAllowedRank"].is_null()),
        "missing a multi-hit LCA with a non-null maxAllowedRank"
    );
    //
    // A found taxon with a null `maxAllowedRank` is not reachable since the
    // BUG-R02/R23 fixes: a non-empty identity-adjusted lineage implies a rank
    // whose cutoff is satisfied. Null `maxAllowedRank` therefore only exists
    // through `taxon: null` here.
    //
    assert!(
        taxa.iter().all(|taxon| !taxon["maxAllowedRank"].is_null()),
        "a found taxon has a null maxAllowedRank; extend the UI contract test"
    );
}
