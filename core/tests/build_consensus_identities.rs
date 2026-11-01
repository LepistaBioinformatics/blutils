mod common;

use blul_core::{
    domain::dtos::{
        consensus_result::ConsensusResult,
        consensus_strategy::ConsensusStrategy, linnaean_ranks::LinnaeanRank,
        parallel_blast_output::ParallelBlastOutput, taxon::Taxon,
        taxonomy_bean::TaxonomyBean,
    },
    use_cases::build_consensus_identities,
};
use common::fixture;
use mycelium_base::utils::errors::MappedErrors;
use std::{fs, path::PathBuf};
use tempfile::TempDir;

const SUBTILIS_TEXT: &str = "d__bacteria;p__bacillota;c__bacilli;o__bacillales;f__bacillaceae;g__bacillus;s__bacillus-subtilis";
const SUBTILIS_NUMERIC: &str =
    "d__2;p__1239;c__91061;o__1385;f__186817;g__1386;s__1423";

/// One 13-column BLAST tabular line.
fn hit(
    query: &str,
    accession: &str,
    taxid: u64,
    perc_identity: f64,
    bit_score: &str,
) -> String {
    format!(
        "{query}\t{accession}\t{taxid}\t{perc_identity}\t500\t0\t0\t1\t500\t1\t500\t1e-100\t{bit_score}\n"
    )
}

fn write_tsv(dir: &TempDir, lines: &[String]) -> PathBuf {
    let path = dir.path().join("blast.tsv");
    fs::write(&path, lines.concat()).unwrap();
    path
}

fn run_with(
    output_file: PathBuf,
    headers: Option<Vec<&str>>,
    taxonomies: PathBuf,
    use_taxid: Option<bool>,
) -> Result<Vec<ConsensusResult>, MappedErrors> {
    build_consensus_identities(
        ParallelBlastOutput {
            output_file,
            headers: headers.map(|h| h.into_iter().map(String::from).collect()),
        },
        &taxonomies,
        Taxon::Bacteria,
        ConsensusStrategy::Cautious,
        use_taxid,
        None,
    )
}

fn run(
    output_file: PathBuf,
    headers: Option<Vec<&str>>,
) -> Vec<ConsensusResult> {
    run_with(
        output_file,
        headers,
        fixture("taxonomies/basic.blutils.json"),
        None,
    )
    .unwrap()
}

fn query_of(result: &ConsensusResult) -> &str {
    match result {
        ConsensusResult::ConsensusFound(found) => &found.query,
        ConsensusResult::NoConsensusFound(missing) => &missing.query,
    }
}

fn sorted(mut results: Vec<ConsensusResult>) -> Vec<ConsensusResult> {
    results.sort_by(|a, b| query_of(a).cmp(query_of(b)));
    results
}

fn taxon_of(result: &ConsensusResult) -> &TaxonomyBean {
    match result {
        ConsensusResult::ConsensusFound(found) => found.taxon.as_ref().unwrap(),
        ConsensusResult::NoConsensusFound(missing) => {
            panic!("unexpected NoConsensusFound for {}", missing.query)
        }
    }
}

fn single_taxon(results: Vec<ConsensusResult>) -> TaxonomyBean {
    assert_eq!(results.len(), 1);
    taxon_of(&results[0]).clone()
}

// FR-JOIN-01, FR-JOIN-04, FR-SCORE-*, AC-02, AC-04 over the shared fixture.
#[test]
fn build_consensus_identities_basic_fixture_classifies_each_query() {
    let results = sorted(run(fixture("blast/basic.tsv"), None));

    assert_eq!(
        results.iter().map(query_of).collect::<Vec<_>>(),
        vec![
            "q1-single-100",
            "q2-single-98",
            "q3-multi-agree",
            "q4-multi-lca-genus"
        ]
    );

    let q1 = taxon_of(&results[0]);
    assert_eq!(q1.reached_rank, LinnaeanRank::Species);
    assert_eq!(q1.identifier, "bacillus-subtilis");
    assert!(q1.single_match);

    let q2 = taxon_of(&results[1]);
    assert_eq!(q2.reached_rank, LinnaeanRank::Genus);
    assert_eq!(q2.identifier, "bacillus");
    // AC-03 (BUG-R02): demoted from species by identity.
    assert!(q2.mutated);
    assert_eq!(q2.max_allowed_rank, Some(LinnaeanRank::Genus));

    // FR-JOIN-04: the three q3 rows are grouped under one query.
    let q3 = taxon_of(&results[2]);
    assert_eq!(q3.reached_rank, LinnaeanRank::Species);
    let q3_beans = q3.consensus_beans.as_ref().unwrap();
    assert_eq!(q3_beans.len(), 1);
    assert_eq!(q3_beans[0].occurrences, 3);
    assert_eq!(
        q3_beans[0].accessions,
        vec!["ACC101.1", "ACC101.2", "ACC101.3"]
    );

    // AC-04: q4 best group (bit 868) diverges at genus → family.
    let q4 = taxon_of(&results[3]);
    assert_eq!(q4.reached_rank, LinnaeanRank::Family);
    assert_eq!(q4.identifier, "bacillaceae");
    let q4_genera = q4
        .consensus_beans
        .as_ref()
        .unwrap()
        .iter()
        .map(|b| b.identifier.as_str())
        .collect::<Vec<_>>();
    assert_eq!(q4_genera, vec!["bacillus", "lysinibacillus"]);
}

// FR-IN-05, FR-JOIN-02
#[test]
fn build_consensus_identities_use_taxid_true_uses_numeric_lineage() {
    let dir = TempDir::new().unwrap();
    let tsv = write_tsv(&dir, &[hit("q1", "ACC101.1", 101, 100.0, "923.0")]);

    let taxon = single_taxon(
        run_with(
            tsv,
            None,
            fixture("taxonomies/basic.blutils.json"),
            Some(true),
        )
        .unwrap(),
    );

    assert_eq!(taxon.identifier, "1423");
    assert_eq!(taxon.taxonomy.as_deref(), Some(SUBTILIS_NUMERIC));
}

// FR-IN-05, FR-JOIN-02
#[test]
fn build_consensus_identities_use_taxid_none_or_false_uses_text_lineage() {
    let dir = TempDir::new().unwrap();
    let tsv = write_tsv(&dir, &[hit("q1", "ACC101.1", 101, 100.0, "923.0")]);

    for use_taxid in [None, Some(false)] {
        let taxon = single_taxon(
            run_with(
                tsv.clone(),
                None,
                fixture("taxonomies/basic.blutils.json"),
                use_taxid,
            )
            .unwrap(),
        );

        assert_eq!(taxon.identifier, "bacillus-subtilis");
        assert_eq!(taxon.taxonomy.as_deref(), Some(SUBTILIS_TEXT));
    }
}

#[test]
fn build_consensus_identities_missing_taxonomies_file_returns_err() {
    let dir = TempDir::new().unwrap();
    let tsv = write_tsv(&dir, &[hit("q1", "ACC101.1", 101, 100.0, "923.0")]);

    let err =
        run_with(tsv, None, dir.path().join("missing.json"), None).unwrap_err();

    assert!(
        err.to_string().contains("Taxonomies file not found"),
        "{err}"
    );
}

#[test]
fn build_consensus_identities_invalid_taxonomies_json_returns_err() {
    let dir = TempDir::new().unwrap();
    let tsv = write_tsv(&dir, &[hit("q1", "ACC101.1", 101, 100.0, "923.0")]);
    let taxonomies = dir.path().join("taxonomies.json");
    fs::write(&taxonomies, "{ not json").unwrap();

    let err = run_with(tsv, None, taxonomies, None).unwrap_err();

    assert!(
        err.to_string().contains(
            "Unexpected error detected on parse `taxonomies` as json"
        ),
        "{err}"
    );
}

#[test]
fn build_consensus_identities_missing_blast_file_returns_err() {
    let dir = TempDir::new().unwrap();

    let err = run_with(
        dir.path().join("missing.tsv"),
        None,
        fixture("taxonomies/basic.blutils.json"),
        None,
    )
    .unwrap_err();

    assert!(
        err.to_string()
            .contains("Unexpected error occurred on load table."),
        "{err}"
    );
}

// Catalog §3.11 [H] "empty → panic" is refuted: an empty TSV loads as an
// empty frame and yields no results.
#[test]
fn build_consensus_identities_empty_blast_file_returns_no_results() {
    let dir = TempDir::new().unwrap();
    let tsv = write_tsv(&dir, &[]);

    assert!(run(tsv, None).is_empty());
}

#[test]
fn build_consensus_identities_empty_blast_file_with_headers_returns_no_consensus(
) {
    let dir = TempDir::new().unwrap();
    let tsv = write_tsv(&dir, &[]);

    let results = run(tsv, Some(vec!["q1"]));

    assert_eq!(results.len(), 1);
    assert!(
        matches!(&results[0], ConsensusResult::NoConsensusFound(m) if m.query == "q1")
    );
}

// BUG-R24: a malformed TSV returns `Err` instead of panicking in
// `load_named_dataframe`.
#[test]
fn build_consensus_identities_malformed_blast_file_returns_err() {
    let dir = TempDir::new().unwrap();
    let tsv = write_tsv(&dir, &["q1\tACC101.1\tnot-a-taxid\n".to_string()]);

    let err =
        run_with(tsv, None, fixture("taxonomies/basic.blutils.json"), None)
            .unwrap_err();

    assert!(
        err.to_string()
            .contains("Unexpected error on parse `blast_output`"),
        "{err}"
    );
}

// AC-01, FR-IN-07, FR-NOHIT-01, FR-NOHIT-02
#[test]
fn build_consensus_identities_header_without_hits_returns_no_consensus() {
    let dir = TempDir::new().unwrap();
    let tsv = write_tsv(&dir, &[hit("q1", "ACC101.1", 101, 100.0, "923.0")]);

    let results = sorted(run(tsv, Some(vec!["q1", "q2"])));

    assert_eq!(results.len(), 2);
    assert!(
        matches!(&results[0], ConsensusResult::ConsensusFound(f) if f.query == "q1")
    );
    assert!(
        matches!(&results[1], ConsensusResult::NoConsensusFound(m) if m.query == "q2")
    );
}

// FR-NOHIT-01: without headers there is no reconciliation.
#[test]
fn build_consensus_identities_without_headers_returns_only_blast_queries() {
    let dir = TempDir::new().unwrap();
    let tsv = write_tsv(&dir, &[hit("q1", "ACC101.1", 101, 100.0, "923.0")]);

    let results = run(tsv, None);

    assert_eq!(results.iter().map(query_of).collect::<Vec<_>>(), vec!["q1"]);
}

// FR-JOIN-03: a row whose taxid is missing from the taxonomies is kept by the
// left join; outside the best bit-score group it is never parsed.
#[test]
fn build_consensus_identities_unknown_taxid_in_lower_group_is_ignored() {
    let dir = TempDir::new().unwrap();
    let tsv = write_tsv(
        &dir,
        &[
            hit("q1", "ACC101.1", 101, 100.0, "923.0"),
            hit("q1", "ACC999.1", 999, 90.0, "400.0"),
        ],
    );

    let taxon = single_taxon(run(tsv, None));

    assert_eq!(taxon.identifier, "bacillus-subtilis");
    assert!(taxon.single_match);
}

// FR-JOIN-03
// BUG-R07: a hit whose taxid is missing from the taxonomies is discarded; a
// query left without usable hits yields `NoConsensusFound` and the other
// queries are still classified.
#[test]
fn build_consensus_identities_unknown_taxid_does_not_abort_other_queries() {
    let dir = TempDir::new().unwrap();
    let tsv = write_tsv(
        &dir,
        &[
            hit("q1", "ACC101.1", 101, 100.0, "923.0"),
            hit("q5", "ACC999.1", 999, 100.0, "923.0"),
        ],
    );

    let results = sorted(run(tsv, None));

    assert_eq!(results.len(), 2);
    assert_eq!(taxon_of(&results[0]).identifier, "bacillus-subtilis");
    assert!(
        matches!(&results[1], ConsensusResult::NoConsensusFound(m) if m.query == "q5")
    );
}

// BUG-R07: the shared fixture's unknown-taxid query yields `NoConsensusFound`
// instead of aborting the run.
#[test]
fn build_consensus_identities_unknown_taxid_fixture_returns_no_consensus() {
    let results = run(fixture("blast/unknown_taxid.tsv"), None);

    assert_eq!(results.len(), 1);
    assert!(matches!(
        &results[0],
        ConsensusResult::NoConsensusFound(m) if m.query == "q5-unknown-taxid"
    ));
}

// BUG-R14: the fractional bit score is kept, so 845.9 wins alone over 845.2.
#[test]
fn build_consensus_identities_fractional_bit_score_is_preserved() {
    let dir = TempDir::new().unwrap();
    let tsv = write_tsv(
        &dir,
        &[
            hit("q1", "ACC101.1", 101, 100.0, "845.9"),
            hit("q1", "ACC104.1", 104, 100.0, "845.2"),
        ],
    );

    let taxon = single_taxon(run(tsv, None));

    assert_eq!(taxon.bit_score, 845.9);
    assert!(taxon.single_match);
    assert_eq!(taxon.identifier, "bacillus-subtilis");
}

// Quotes are stripped from the query name (`mod.rs` `replace("\"", "")`).
#[test]
fn build_consensus_identities_quotes_in_query_name_are_removed() {
    let dir = TempDir::new().unwrap();
    let tsv = write_tsv(&dir, &[hit("q\"7", "ACC101.1", 101, 100.0, "923.0")]);

    let results = run(tsv, None);

    assert_eq!(results.iter().map(query_of).collect::<Vec<_>>(), vec!["q7"]);
}
