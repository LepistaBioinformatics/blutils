use crate::domain::dtos::{
    consensus_result::{
        ConsensusBean, ConsensusResult, QueryWithConsensus,
        QueryWithoutConsensus,
    },
    linnaean_ranks::{
        InterpolatedIdentity, LinnaeanRank, RankedLinnaeanIdentity::*,
    },
    taxonomy_bean::TaxonomyBean,
};

/// Build the final consensus taxon for the lineage `taxonomy` cut at
/// `bean_index`.
///
/// The lineage is truncated at `bean_index` and then by the identity cutoffs.
/// When no rank satisfies the identity, `NoConsensusFound` is returned.
pub(super) fn build_blast_consensus_identity(
    query: String,
    mut bean: TaxonomyBean,
    max_allowed_identity: f64,
    bean_index: usize,
    taxonomy: Vec<TaxonomyBean>,
    interpolated_taxonomy: InterpolatedIdentity,
    consensus_beans: Option<Vec<ConsensusBean>>,
) -> ConsensusResult {
    if taxonomy.get(bean_index).is_none() {
        panic!("No taxonomy found for bean at index: {}", bean_index);
    }

    //
    // Truncate the lineage at the target index and then by identity.
    //
    // BUG-R22: the lineage is always cut at `bean_index`, so a level where
    // all hits agree never reports ranks deeper than the evaluated ones.
    //
    let adjusted_taxonomy = interpolated_taxonomy
        .get_adjusted_taxonomy_by_identity(
            max_allowed_identity,
            taxonomy[..=bean_index].to_vec(),
        );

    //
    // BUG-R23: an identity below every cutoff justifies no rank at all.
    //
    let last_taxonomy = match adjusted_taxonomy.last() {
        Some(last) => last.to_owned(),
        None => {
            return ConsensusResult::NoConsensusFound(QueryWithoutConsensus {
                query,
            })
        }
    };

    //
    // Update the rank of the bean according to the interpolated taxonomy.
    //
    bean.max_allowed_rank = match interpolated_taxonomy
        .get_rank_adjusted_by_identity(max_allowed_identity)
    {
        Some(identity_adjusted_rank) => match identity_adjusted_rank {
            DefaultRank(rank, _) => Some(rank),
            NonDefaultRank(rank, _) => Some(LinnaeanRank::Other(rank)),
        },
        None => None,
    };

    //
    // BUG-R02/R21: the bean is mutated when the identity truncation demoted
    // its rank, so the check runs after the truncation (AC-03, FR-IDENT-05).
    //
    bean.mutated = bean.reached_rank != last_taxonomy.reached_rank;
    bean.identifier = last_taxonomy.identifier.to_owned();
    bean.reached_rank = last_taxonomy.reached_rank.to_owned();
    bean.taxonomy = Some(
        adjusted_taxonomy
            .into_iter()
            .map(|i| i.taxonomy_to_string())
            .collect::<Vec<String>>()
            .join(";"),
    );

    //
    // Fold consensus beans by rank and identity to aggregate the occurrences
    // of each taxon.
    //
    let mut consensus_beans =
        ConsensusBean::fold_consensus_list(consensus_beans.unwrap_or_default());

    //
    // Sort consensus beans by occurrences and identifier.
    //
    if !consensus_beans.is_empty() {
        consensus_beans.sort_by(|a, b| {
            a.occurrences
                .partial_cmp(&b.occurrences)
                .unwrap_or(std::cmp::Ordering::Equal)
                .reverse()
                .then(
                    a.identifier
                        .partial_cmp(&b.identifier)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });

        bean.consensus_beans = Some(consensus_beans);
    }

    ConsensusResult::ConsensusFound(QueryWithConsensus {
        query,
        taxon: Some(bean),
        run_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::{
        blast_result::BlastResultRow, taxon::Taxon, taxonomy_bean::Taxonomy,
    };
    use crate::test_utils::row;

    const SUBTILIS: &str = "d__bacteria;p__bacillota;c__bacilli;o__bacillales;f__bacillaceae;g__bacillus;s__bacillus-subtilis";
    const GENUS: &str = "d__bacteria;p__bacillota;c__bacilli;o__bacillales;f__bacillaceae;g__bacillus";

    fn lineage_of(literal: &str) -> Vec<TaxonomyBean> {
        let parsed: BlastResultRow =
            row("acc", 1, 100.0, 500, literal).parse_taxonomy().unwrap();
        match parsed.taxonomy {
            Taxonomy::Parsed(beans) => beans,
            Taxonomy::Literal(_) => unreachable!(),
        }
    }

    fn interp_of(lineage: &[TaxonomyBean]) -> InterpolatedIdentity {
        InterpolatedIdentity::new(
            Taxon::Bacteria,
            lineage.iter().map(|b| b.reached_rank.clone()).collect(),
            None,
        )
        .unwrap()
    }

    fn bean(identifier: &str, accession: &str) -> ConsensusBean {
        ConsensusBean {
            rank: LinnaeanRank::Genus,
            identifier: identifier.to_string(),
            occurrences: 0,
            taxonomy: None,
            accessions: vec![accession.to_string()],
        }
    }

    fn build_result(
        lineage: &[TaxonomyBean],
        input: TaxonomyBean,
        identity: f64,
        index: usize,
        beans: Option<Vec<ConsensusBean>>,
    ) -> ConsensusResult {
        build_blast_consensus_identity(
            "q1".to_string(),
            input,
            identity,
            index,
            lineage.to_vec(),
            interp_of(lineage),
            beans,
        )
    }

    fn build(
        lineage: &[TaxonomyBean],
        input: TaxonomyBean,
        identity: f64,
        index: usize,
        beans: Option<Vec<ConsensusBean>>,
    ) -> TaxonomyBean {
        match build_result(lineage, input, identity, index, beans) {
            ConsensusResult::ConsensusFound(found) => found.taxon.unwrap(),
            ConsensusResult::NoConsensusFound(missing) => {
                panic!("unexpected NoConsensusFound for {}", missing.query)
            }
        }
    }

    #[test]
    #[should_panic(expected = "No taxonomy found for bean at index: 7")]
    fn build_blast_consensus_identity_bean_index_out_of_range_panics() {
        let lineage = lineage_of(SUBTILIS);
        build(&lineage, lineage[6].clone(), 100.0, 7, None);
    }

    // FR-BEAN-02: no beans → `consensus_beans` stays None.
    #[test]
    fn build_blast_consensus_identity_without_beans_keeps_consensus_beans_none()
    {
        let lineage = lineage_of(SUBTILIS);

        let from_none = build(&lineage, lineage[6].clone(), 100.0, 6, None);
        let from_empty =
            build(&lineage, lineage[6].clone(), 100.0, 6, Some(vec![]));

        assert_eq!(from_none.consensus_beans, None);
        assert_eq!(from_empty.consensus_beans, None);
    }

    // FR-BEAN-01, FR-BEAN-02: folded, then occurrences desc, identifier asc.
    #[test]
    fn build_blast_consensus_identity_beans_sorted_by_occurrences_then_identifier(
    ) {
        let lineage = lineage_of(SUBTILIS);
        let taxon = build(
            &lineage,
            lineage[4].clone(),
            100.0,
            4,
            Some(vec![
                bean("beta", "b1"),
                bean("zeta", "z1"),
                bean("alpha", "a1"),
                bean("zeta", "z2"),
            ]),
        );

        let summary = taxon
            .consensus_beans
            .unwrap()
            .into_iter()
            .map(|b| (b.identifier, b.occurrences, b.accessions))
            .collect::<Vec<_>>();
        assert_eq!(
            summary,
            vec![
                (
                    "zeta".to_string(),
                    2,
                    vec!["z1".to_string(), "z2".to_string()]
                ),
                ("alpha".to_string(), 1, vec!["a1".to_string()]),
                ("beta".to_string(), 1, vec!["b1".to_string()]),
            ]
        );
    }

    // FR-IDENT-05 (BUG-R01): an identity above every cutoff allows the
    // deepest rank; nothing is truncated, so the bean is not mutated whatever
    // the input flag was.
    #[test]
    fn build_blast_consensus_identity_identity_above_all_cutoffs_is_not_mutated(
    ) {
        let lineage = lineage_of(SUBTILIS);

        for mutated in [true, false] {
            let input = TaxonomyBean {
                mutated,
                ..lineage[6].clone()
            };
            let taxon = build(&lineage, input, 100.0, 6, None);

            assert_eq!(taxon.max_allowed_rank, Some(LinnaeanRank::Species));
            assert_eq!(taxon.reached_rank, LinnaeanRank::Species);
            assert!(!taxon.mutated);
        }
    }

    // FR-IDENT-05 (BUG-R01): a genus bean at 98% is allowed up to genus and
    // is not demoted.
    #[test]
    fn build_blast_consensus_identity_rank_within_allowed_is_not_mutated() {
        let lineage = lineage_of(SUBTILIS);

        let genus = build(&lineage, lineage[5].clone(), 98.0, 5, None);

        assert_eq!(genus.max_allowed_rank, Some(LinnaeanRank::Genus));
        assert_eq!(genus.reached_rank, LinnaeanRank::Genus);
        assert!(!genus.mutated);
    }

    // BUG-R21: `mutated` is evaluated after the identity truncation: a
    // species bean at 98% ends at genus, `max_allowed_rank` Genus and
    // `mutated` true (AC-03).
    #[test]
    fn build_blast_consensus_identity_identity_truncation_marks_mutated() {
        let lineage = lineage_of(SUBTILIS);
        let taxon = build(&lineage, lineage[6].clone(), 98.0, 6, None);

        assert_eq!(taxon.reached_rank, LinnaeanRank::Genus);
        assert_eq!(taxon.identifier, "bacillus");
        assert_eq!(taxon.taxonomy.as_deref(), Some(GENUS));
        assert_eq!(taxon.max_allowed_rank, Some(LinnaeanRank::Genus));
        assert!(taxon.mutated);
    }

    // FR-IDENT-02: a non-default rank maps to `LinnaeanRank::Other`.
    // BUG-R01: 100% satisfies the interpolated strain cutoff (100), so strain
    // is the maximum allowed rank; the species bean is not demoted.
    #[test]
    fn build_blast_consensus_identity_non_default_rank_maps_to_other() {
        let lineage = lineage_of(&format!("{SUBTILIS};strain__x1"));
        let taxon = build(&lineage, lineage[6].clone(), 100.0, 6, None);

        assert_eq!(
            taxon.max_allowed_rank,
            Some(LinnaeanRank::Other("strain".to_string()))
        );
        assert!(!taxon.mutated);
        assert_eq!(taxon.reached_rank, LinnaeanRank::Species);
    }

    // FR-MULTI-05: the lineage is cut at `bean_index`.
    #[test]
    fn build_blast_consensus_identity_truncates_at_bean_index() {
        let lineage = lineage_of(SUBTILIS);
        let taxon = build(
            &lineage,
            lineage[4].clone(),
            100.0,
            4,
            Some(vec![bean("alpha", "a"), bean("beta", "b")]),
        );

        assert_eq!(taxon.reached_rank, LinnaeanRank::Family);
        assert_eq!(taxon.identifier, "bacillaceae");
        assert_eq!(
            taxon.taxonomy.as_deref(),
            Some("d__bacteria;p__bacillota;c__bacilli;o__bacillales;f__bacillaceae")
        );
    }

    // BUG-R22: a single folded bean no longer skips `bean_index`; the lineage
    // stops at the evaluated level.
    #[test]
    fn build_blast_consensus_identity_one_folded_bean_truncates_at_bean_index()
    {
        let lineage = lineage_of(SUBTILIS);
        let taxon = build(
            &lineage,
            lineage[4].clone(),
            100.0,
            4,
            Some(vec![bean("bacillaceae", "a"), bean("bacillaceae", "b")]),
        );

        assert_eq!(taxon.reached_rank, LinnaeanRank::Family);
        assert_eq!(taxon.identifier, "bacillaceae");
        assert_eq!(
            taxon.taxonomy.as_deref(),
            Some("d__bacteria;p__bacillota;c__bacilli;o__bacillales;f__bacillaceae")
        );
    }

    // BUG-R23: an identity below every cutoff justifies no rank, so no
    // consensus is reported.
    #[test]
    fn build_blast_consensus_identity_identity_below_all_cutoffs_returns_no_consensus(
    ) {
        let lineage = lineage_of(SUBTILIS);

        match build_result(&lineage, lineage[4].clone(), 50.0, 4, None) {
            ConsensusResult::NoConsensusFound(missing) => {
                assert_eq!(missing.query, "q1")
            }
            other => panic!("expected NoConsensusFound, got {other:?}"),
        }
    }
}
