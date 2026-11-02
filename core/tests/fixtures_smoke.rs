mod common;

use blul_core::{
    domain::dtos::{
        consensus_result::ConsensusResult, consensus_strategy::ConsensusStrategy,
        parallel_blast_output::ParallelBlastOutput, taxon::Taxon,
    },
    use_cases::build_consensus_identities,
};
use common::fixture;

#[test]
fn common_fixtures_load_into_one_result_per_query() {
    let output = ParallelBlastOutput {
        output_file: fixture("blast/basic.tsv"),
        headers: None,
    };

    let results = build_consensus_identities(
        output,
        &fixture("taxonomies/basic.blutils.json"),
        Taxon::Bacteria,
        ConsensusStrategy::Cautious,
        None,
        None,
    )
    .expect("fixtures should be consumable by the consensus pipeline");

    let mut queries: Vec<String> = results
        .iter()
        .map(|result| match result {
            ConsensusResult::ConsensusFound(found) => found.query.clone(),
            ConsensusResult::NoConsensusFound(missing) => {
                panic!("unexpected NoConsensusFound for {}", missing.query)
            }
        })
        .collect();
    queries.sort();

    assert_eq!(
        queries,
        vec![
            "q1-single-100",
            "q2-single-98",
            "q3-multi-agree",
            "q4-multi-lca-genus"
        ]
    );
}
