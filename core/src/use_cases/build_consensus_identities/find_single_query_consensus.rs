use super::{
    build_blast_consensus_identity, find_multi_taxa_consensus,
    force_parsed_taxonomy,
};
use crate::domain::dtos::{
    blast_result::BlastResultRow,
    consensus_result::{ConsensusBean, ConsensusResult, QueryWithoutConsensus},
    consensus_strategy::ConsensusStrategy,
    linnaean_ranks::InterpolatedIdentity,
    taxon::{CustomTaxon, Taxon},
    taxonomy_bean::Taxonomy,
};

use mycelium_base::utils::errors::MappedErrors;
use tracing::warn;

pub(super) fn find_single_query_consensus(
    query: String,
    result: Vec<BlastResultRow>,
    taxon: Taxon,
    strategy: ConsensusStrategy,
    custom_taxon_values: Option<CustomTaxon>,
) -> Result<ConsensusResult, MappedErrors> {
    let no_consensus = QueryWithoutConsensus {
        query: query.to_owned(),
    };

    // ? -----------------------------------------------------------------------
    // ? Select the best bit-score group
    //
    // Only the hits sharing the highest bit-score are evaluated (FR-SCORE-02).
    //
    // BUG-R14: bit-scores are compared as exact floats, without truncation.
    //
    // ? -----------------------------------------------------------------------

    let best_score = match result
        .iter()
        .map(|record| record.bit_score)
        .max_by(|a, b| a.total_cmp(b))
    {
        Some(score) => score,
        None => return Ok(ConsensusResult::NoConsensusFound(no_consensus)),
    };

    //
    // BUG-R07: hits whose lineage cannot be parsed (e.g. a taxid missing from
    // the taxonomies file) carry no taxonomic evidence and are discarded.
    //
    let bit_score_matches = result
        .into_iter()
        .filter(|record| record.bit_score.to_bits() == best_score.to_bits())
        .filter_map(|mut record| match record.parse_taxonomy() {
            Ok(parsed) => Some(parsed),
            Err(err) => {
                warn!(
                    "Ignoring hit `{}` of query `{}`: {err}",
                    record.subject_accession, query
                );

                None
            }
        })
        .collect::<Vec<BlastResultRow>>();

    match bit_score_matches.len() {
        //
        // BUG-R07/BUG-R25: no hit of the best group has a usable lineage.
        //
        0 => Ok(ConsensusResult::NoConsensusFound(no_consensus)),
        //
        // Fetch the lower taxonomic rank case only one record returned.
        //
        1 => {
            let target_blast_match = &bit_score_matches[0];
            //
            // This action prevents the program to panic when the taxonomy
            // is not already parsed.
            //
            let taxonomies =
                force_parsed_taxonomy(target_blast_match.taxonomy.to_owned())?;
            //
            // Generate interpolated identities for the taxon.
            //
            let interpolated_identities = InterpolatedIdentity::new(
                taxon.to_owned(),
                taxonomies
                    .clone()
                    .into_iter()
                    .map(|bean| bean.reached_rank)
                    .collect(),
                custom_taxon_values,
            )?;
            //
            // Fetch the adjusted taxonomy based on the interpolated identities.
            //
            let identity_adjusted_taxonomy = interpolated_identities
                .get_adjusted_taxonomy_by_identity(
                    target_blast_match.perc_identity,
                    taxonomies.to_owned(),
                );
            //
            // BUG-R08: an identity below every cutoff justifies no rank.
            //
            let (target_bean, input_bean) =
                match (identity_adjusted_taxonomy.last(), taxonomies.last()) {
                    (Some(target), Some(input)) => {
                        (target.to_owned(), input.to_owned())
                    }
                    _ => {
                        return Ok(ConsensusResult::NoConsensusFound(
                            no_consensus,
                        ))
                    }
                };
            //
            // Initialize the consensus bean.
            //
            let consensus_bean = ConsensusBean::from_taxonomy_bean(
                target_bean,
                Some(target_blast_match.subject_accession.to_owned()),
                Taxonomy::taxonomy_beans_to_string(taxonomies.to_owned()),
            );
            //
            // BUG-R02: the final taxon is built like the multi path, so
            // `mutated` and `max_allowed_rank` reflect the truncation.
            //
            let mut consensus = build_blast_consensus_identity(
                query,
                input_bean,
                target_blast_match.perc_identity,
                taxonomies.len() - 1,
                taxonomies,
                interpolated_identities,
                Some(vec![consensus_bean]),
            )?;

            if let ConsensusResult::ConsensusFound(found) = &mut consensus {
                if let Some(taxon) = found.taxon.as_mut() {
                    taxon.single_match = true;
                }
            }

            Ok(consensus)
        }
        //
        // Fetch the lower taxonomic rank case more than one record returned.
        //
        _ => find_multi_taxa_consensus(
            bit_score_matches,
            taxon,
            no_consensus,
            strategy,
            custom_taxon_values,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::{
        linnaean_ranks::LinnaeanRank, taxonomy_bean::TaxonomyBean,
    };
    use crate::test_utils::row;

    const SUBTILIS: &str = "d__bacteria;p__bacillota;c__bacilli;o__bacillales;f__bacillaceae;g__bacillus;s__bacillus-subtilis";
    const CEREUS: &str = "d__bacteria;p__bacillota;c__bacilli;o__bacillales;f__bacillaceae;g__bacillus;s__bacillus-cereus";
    const SPHAERICUS: &str = "d__bacteria;p__bacillota;c__bacilli;o__bacillales;f__bacillaceae;g__lysinibacillus;s__lysinibacillus-sphaericus";

    fn run(rows: Vec<BlastResultRow>) -> ConsensusResult {
        find_single_query_consensus(
            "q1".to_string(),
            rows,
            Taxon::Bacteria,
            ConsensusStrategy::Cautious,
            None,
        )
        .unwrap()
    }

    fn found_taxon(result: ConsensusResult) -> TaxonomyBean {
        match result {
            ConsensusResult::ConsensusFound(found) => {
                assert_eq!(found.query, "q1");
                found.taxon.expect("consensus without taxon")
            }
            ConsensusResult::NoConsensusFound(missing) => {
                panic!("unexpected NoConsensusFound for {}", missing.query)
            }
        }
    }

    // FR-SCORE-03
    #[test]
    fn find_single_query_consensus_empty_results_returns_no_consensus() {
        match run(vec![]) {
            ConsensusResult::NoConsensusFound(missing) => {
                assert_eq!(missing.query, "q1")
            }
            other => panic!("expected NoConsensusFound, got {other:?}"),
        }
    }

    // FR-SCORE-01, FR-SCORE-02: only the best bit-score group is evaluated;
    // the three lower hits (which would form a multi group) are discarded.
    #[test]
    fn find_single_query_consensus_best_bit_score_single_hit_ignores_lower_groups(
    ) {
        let taxon = found_taxon(run(vec![
            row("LOW-1", 102, 100.0, 400, CEREUS),
            row("LOW-2", 104, 100.0, 400, SPHAERICUS),
            row("BEST", 101, 100.0, 500, SUBTILIS),
            row("LOW-3", 102, 100.0, 400, CEREUS),
        ]));

        assert!(taxon.single_match);
        assert_eq!(taxon.identifier, "bacillus-subtilis");
        assert_eq!(taxon.bit_score, 500.0);
        let beans = taxon.consensus_beans.unwrap();
        assert_eq!(beans.len(), 1);
        assert_eq!(beans[0].accessions, vec!["BEST".to_string()]);
    }

    // FR-SCORE-02: lower bit-score groups are never parsed, so an invalid
    // lineage there does not affect the result.
    #[test]
    fn find_single_query_consensus_invalid_lineage_in_lower_group_is_not_parsed(
    ) {
        let taxon = found_taxon(run(vec![
            row("BEST", 101, 100.0, 500, SUBTILIS),
            row("BROKEN", 999, 100.0, 100, "null"),
        ]));

        assert_eq!(taxon.identifier, "bacillus-subtilis");
    }

    // FR-SCORE-01: the best group with more than one hit goes to the multi
    // path even when a lower group has a single hit.
    #[test]
    fn find_single_query_consensus_best_group_with_many_hits_uses_multi_path() {
        let taxon = found_taxon(run(vec![
            row("SINGLE-LOW", 101, 100.0, 300, SUBTILIS),
            row("A", 101, 100.0, 500, SUBTILIS),
            row("B", 104, 100.0, 500, SPHAERICUS),
        ]));

        assert!(!taxon.single_match);
        assert_eq!(taxon.reached_rank, LinnaeanRank::Family);
        assert_eq!(taxon.identifier, "bacillaceae");
    }

    // AC-02, FR-SINGLE-01, FR-SINGLE-02, FR-SINGLE-03, FR-SINGLE-04
    #[test]
    fn find_single_query_consensus_single_hit_100_returns_full_lineage_at_species(
    ) {
        let taxon =
            found_taxon(run(vec![row("ACC1", 101, 100.0, 923, SUBTILIS)]));

        assert_eq!(taxon.reached_rank, LinnaeanRank::Species);
        assert_eq!(taxon.identifier, "bacillus-subtilis");
        assert!(taxon.single_match);
        assert!(!taxon.mutated);
        // BUG-R01/BUG-R02: 100% allows the deepest rank (was None).
        assert_eq!(taxon.max_allowed_rank, Some(LinnaeanRank::Species));
        assert_eq!(taxon.perc_identity, 100.0);
        assert_eq!(taxon.bit_score, 923.0);
        assert_eq!(taxon.taxonomy.as_deref(), Some(SUBTILIS));

        let beans = taxon.consensus_beans.unwrap();
        assert_eq!(beans.len(), 1);
        assert_eq!(beans[0].rank, LinnaeanRank::Species);
        assert_eq!(beans[0].identifier, "bacillus-subtilis");
        assert_eq!(beans[0].occurrences, 1);
        assert_eq!(beans[0].accessions, vec!["ACC1".to_string()]);
        assert_eq!(beans[0].taxonomy.as_deref(), Some(SUBTILIS));
    }

    // AC-03, FR-SINGLE-03
    // BUG-R02: the single path sets `mutated` and `max_allowed_rank` after
    // the identity truncation.
    #[test]
    fn find_single_query_consensus_single_hit_98_marks_mutated() {
        let taxon =
            found_taxon(run(vec![row("ACC1", 101, 98.0, 850, SUBTILIS)]));

        assert_eq!(taxon.reached_rank, LinnaeanRank::Genus);
        assert_eq!(taxon.identifier, "bacillus");
        assert_eq!(
            taxon.taxonomy.as_deref(),
            Some("d__bacteria;p__bacillota;c__bacilli;o__bacillales;f__bacillaceae;g__bacillus")
        );
        assert!(taxon.single_match);
        assert!(taxon.mutated);
        assert!(taxon.max_allowed_rank.is_some());
        assert_eq!(taxon.max_allowed_rank, Some(LinnaeanRank::Genus));

        // The bean keeps the full (untruncated) lineage of the hit.
        let beans = taxon.consensus_beans.unwrap();
        assert_eq!(beans.len(), 1);
        assert_eq!(beans[0].rank, LinnaeanRank::Genus);
        assert_eq!(beans[0].identifier, "bacillus");
        assert_eq!(beans[0].taxonomy.as_deref(), Some(SUBTILIS));
    }

    // BUG-R08: identity below every cutoff (Bacteria domain = 60) yields no
    // consensus instead of panicking.
    #[test]
    fn find_single_query_consensus_identity_below_all_cutoffs_returns_no_consensus(
    ) {
        match run(vec![row("ACC1", 101, 55.0, 100, SUBTILIS)]) {
            ConsensusResult::NoConsensusFound(missing) => {
                assert_eq!(missing.query, "q1")
            }
            other => panic!("expected NoConsensusFound, got {other:?}"),
        }
    }

    // BUG-R08: NaN identity satisfies no cutoff and yields no consensus.
    #[test]
    fn find_single_query_consensus_nan_identity_returns_no_consensus() {
        assert!(matches!(
            run(vec![row("ACC1", 101, f64::NAN, 100, SUBTILIS)]),
            ConsensusResult::NoConsensusFound(_)
        ));
    }

    // BUG-R07 (unit level): an unparseable lineage (e.g. the "null" produced
    // by the left join for unknown taxids) is discarded; with no other hit in
    // the best group there is no consensus.
    #[test]
    fn find_single_query_consensus_unparseable_lineage_returns_no_consensus() {
        assert!(matches!(
            run(vec![row("ACC1", 999, 100.0, 923, "null")]),
            ConsensusResult::NoConsensusFound(_)
        ));
    }

    // BUG-R07: an unparseable hit in the best group is discarded and the
    // remaining hit of that group is classified alone.
    #[test]
    fn find_single_query_consensus_unparseable_lineage_in_best_group_is_discarded(
    ) {
        let taxon = found_taxon(run(vec![
            row("BROKEN", 999, 100.0, 923, "null"),
            row("ACC1", 101, 100.0, 923, SUBTILIS),
            row("LOW", 104, 100.0, 500, SPHAERICUS),
        ]));

        assert!(taxon.single_match);
        assert_eq!(taxon.identifier, "bacillus-subtilis");
        assert_eq!(
            taxon.consensus_beans.unwrap()[0].accessions,
            vec!["ACC1".to_string()]
        );
    }

    // BUG-R14: bit-scores are grouped by their exact value; 845.9 wins alone
    // over 845.2 and is reported unchanged.
    #[test]
    fn find_single_query_consensus_fractional_bit_scores_are_not_truncated() {
        let mut best = row("BEST", 101, 100.0, 0, SUBTILIS);
        best.bit_score = 845.9;
        let mut other = row("OTHER", 104, 100.0, 0, SPHAERICUS);
        other.bit_score = 845.2;

        let taxon: TaxonomyBean = found_taxon(run(vec![other, best]));

        assert!(taxon.single_match);
        assert_eq!(taxon.identifier, "bacillus-subtilis");
        assert_eq!(taxon.bit_score, 845.9);
    }
}
