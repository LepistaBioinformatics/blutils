use super::{build_blast_consensus_identity, force_parsed_taxonomy};
use crate::domain::dtos::{
    blast_result::BlastResultRow,
    consensus_result::{ConsensusBean, ConsensusResult, QueryWithoutConsensus},
    consensus_strategy::ConsensusStrategy,
    linnaean_ranks::InterpolatedIdentity,
    taxon::{CustomTaxon, Taxon},
    taxonomy_bean::{Taxonomy, TaxonomyBean},
};

use mycelium_base::utils::errors::MappedErrors;
use std::collections::HashSet;

/// Find the consensus among Blast results with multiple output.
///
/// In some cases blast results returns a list if records with the same percent
/// identity and bit-score. In this cases this logic could be applied to solve
/// the problem.
pub(super) fn find_multi_taxa_consensus(
    records: Vec<BlastResultRow>,
    taxon: Taxon,
    no_consensus_option: QueryWithoutConsensus,
    strategy: ConsensusStrategy,
    custom_taxon_values: Option<CustomTaxon>,
) -> Result<ConsensusResult, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Collect the reference taxonomy vector
    //
    // The taxonomies vector contain elements of the reference taxonomy given
    // the selected strategy. The `Cautious` strategy selects the shortest
    // taxonomic vector as a reference. Otherwise (`Relaxed` strategy), the
    // longest taxonomic vector is selected.
    //
    // ? -----------------------------------------------------------------------

    let mut sorted_records = records.to_owned();

    sorted_records.sort_by(|a, b| {
        let a_taxonomy = force_parsed_taxonomy(a.taxonomy.to_owned());
        let b_taxonomy = force_parsed_taxonomy(b.taxonomy.to_owned());
        a_taxonomy
            .len()
            .cmp(&b_taxonomy.len())
            .then(
                a.perc_identity
                    .partial_cmp(&b.perc_identity)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(a.align_length.cmp(&b.align_length))
            .then(a.subject_accession.cmp(&b.subject_accession))
    });

    //
    // The reference taxonomy is the longest or shortest taxonomy vector, given
    // the selected strategy.
    //
    let reference_taxonomy = match match strategy {
        ConsensusStrategy::Cautious => sorted_records.first(),
        ConsensusStrategy::Relaxed => sorted_records.last(),
    } {
        Some(reference) => force_parsed_taxonomy(reference.taxonomy.to_owned()),
        None => {
            return Ok(ConsensusResult::NoConsensusFound(no_consensus_option))
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Build the bi-dimensional
    //
    // Each position of the vector contain a vector of the `TaxonomyElement`
    // type.
    //
    // ? -----------------------------------------------------------------------

    let sorted_taxonomies = sorted_records
        .iter()
        .map(|i| force_parsed_taxonomy(i.taxonomy.to_owned()))
        .collect::<Vec<Vec<TaxonomyBean>>>();

    // ? -----------------------------------------------------------------------
    // ? Initialize the final response
    //
    // It is replaced by the consensus of the deepest evaluated level.
    //
    // ? -----------------------------------------------------------------------

    let mut final_result =
        ConsensusResult::NoConsensusFound(no_consensus_option.to_owned());

    // ? -----------------------------------------------------------------------
    // ? Generate interpolated identities
    //
    // Initialize the interpolated identities. Interpolated identities contains
    // the full lineage of the records to be tested including the interpolated
    // identity percentages.
    //
    // ? -----------------------------------------------------------------------

    let interpolated_identities = InterpolatedIdentity::new(
        taxon.to_owned(),
        reference_taxonomy
            .clone()
            .into_iter()
            .map(|bean| bean.reached_rank)
            .collect(),
        custom_taxon_values,
    )?;

    if interpolated_identities.interpolation().len() != reference_taxonomy.len()
    {
        panic!(
            "Interpolated identities length is not equal to reference taxonomy length"
        );
    }

    // ? -----------------------------------------------------------------------
    // ? Try to update the final response
    //
    // Here the reference taxonomies (the longest or shortest) are used to
    // filter children taxonomies.
    //
    // ? -----------------------------------------------------------------------

    for (index, ref_taxonomy) in reference_taxonomy.iter().enumerate() {
        //
        // The level taxonomic record is a tuple containing the taxonomies and
        // the records of the same index.
        //
        // BUG-R06: every co-best hit that reaches this level is evaluated,
        // so the Relaxed reference is walked beyond the shortest lineage.
        //
        let level_max_taxonomy = sorted_taxonomies
            .iter()
            .zip(sorted_records.iter())
            .filter(|(taxonomy, _)| index < taxonomy.len());

        //
        // Collect the taxonomies of the same level.
        //
        let level_taxonomy = level_max_taxonomy
            .to_owned()
            .map(|(taxonomy, _)| {
                format!(
                    "{rank}{identifier}",
                    rank = taxonomy[index].reached_rank.to_string(),
                    identifier = &taxonomy[index].identifier
                )
            })
            .collect::<HashSet<String>>();

        if level_taxonomy.is_empty() {
            continue;
        }

        //
        // If the level taxonomies has more than one element, try to find
        // the consensus between them.
        //
        let consensus_beans = level_max_taxonomy
            .to_owned()
            .map(|(taxonomy, record)| {
                ConsensusBean::from_taxonomy_bean(
                    taxonomy[index].to_owned(),
                    Some(record.subject_accession.to_owned()),
                    Taxonomy::taxonomy_beans_to_string(taxonomy.to_owned()),
                )
            })
            .collect::<Vec<ConsensusBean>>();

        if level_taxonomy.len() > 1 {
            //
            // BUG-R03: hits diverging at the first rank share no ancestor.
            //
            if index == 0 {
                final_result = ConsensusResult::NoConsensusFound(
                    no_consensus_option.to_owned(),
                );

                break;
            }

            let target_index = index - 1;
            let max_pert_identity = level_max_taxonomy
                .clone()
                .map(|(_, i)| i.perc_identity)
                .fold(0.0, |acc, i| if i > acc { i } else { acc });

            //
            // Build the consensus identity based on the multi-level taxonomy.
            //
            final_result = build_blast_consensus_identity(
                no_consensus_option.query.to_owned(),
                reference_taxonomy[target_index].to_owned(),
                max_pert_identity,
                target_index,
                reference_taxonomy.to_owned(),
                interpolated_identities.to_owned(),
                Some(consensus_beans),
            );

            break;
        }

        final_result = build_blast_consensus_identity(
            no_consensus_option.query.to_owned(),
            ref_taxonomy.to_owned(),
            ref_taxonomy.perc_identity,
            index,
            reference_taxonomy.to_owned(),
            interpolated_identities.to_owned(),
            Some(consensus_beans),
        )
    }

    Ok(final_result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::linnaean_ranks::LinnaeanRank;
    use crate::test_utils::row;

    const FAMILY: &str =
        "d__bacteria;p__bacillota;c__bacilli;o__bacillales;f__bacillaceae";

    fn lineage(genus: &str, species: &str) -> String {
        format!("{FAMILY};g__{genus};s__{species}")
    }

    fn parsed(
        accession: &str,
        perc_identity: f64,
        lineage: &str,
    ) -> BlastResultRow {
        row(accession, 1, perc_identity, 500, lineage)
            .parse_taxonomy()
            .unwrap()
    }

    fn run(
        records: Vec<BlastResultRow>,
        strategy: ConsensusStrategy,
    ) -> ConsensusResult {
        find_multi_taxa_consensus(
            records,
            Taxon::Bacteria,
            QueryWithoutConsensus {
                query: "q1".to_string(),
            },
            strategy,
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

    fn bean_summary(taxon: &TaxonomyBean) -> Vec<(String, i32, Vec<String>)> {
        taxon
            .consensus_beans
            .clone()
            .unwrap()
            .into_iter()
            .map(|b| (b.identifier, b.occurrences, b.accessions))
            .collect()
    }

    #[test]
    fn find_multi_taxa_consensus_empty_records_returns_no_consensus() {
        match run(vec![], ConsensusStrategy::Cautious) {
            ConsensusResult::NoConsensusFound(missing) => {
                assert_eq!(missing.query, "q1")
            }
            other => panic!("expected NoConsensusFound, got {other:?}"),
        }
    }

    // FR-MULTI-03, FR-MULTI-04
    #[test]
    fn find_multi_taxa_consensus_all_hits_agree_returns_species_with_one_bean()
    {
        let subtilis = lineage("bacillus", "bacillus-subtilis");
        let taxon = found_taxon(run(
            vec![
                parsed("c", 100.0, &subtilis),
                parsed("a", 100.0, &subtilis),
                parsed("b", 100.0, &subtilis),
            ],
            ConsensusStrategy::Cautious,
        ));

        assert_eq!(taxon.reached_rank, LinnaeanRank::Species);
        assert_eq!(taxon.identifier, "bacillus-subtilis");
        assert_eq!(taxon.taxonomy.as_deref(), Some(subtilis.as_str()));
        assert!(!taxon.single_match);
        assert!(!taxon.mutated);
        // BUG-R01: 100% allows the deepest rank (was None).
        assert_eq!(taxon.max_allowed_rank, Some(LinnaeanRank::Species));
        assert_eq!(
            bean_summary(&taxon),
            vec![(
                "bacillus-subtilis".to_string(),
                3,
                vec!["a".to_string(), "b".to_string(), "c".to_string()]
            )]
        );
    }

    // AC-04, FR-MULTI-05, FR-BEAN-02: divergence at genus → family (LCA);
    // beans list the genera sorted by occurrences desc, then identifier asc.
    #[test]
    fn find_multi_taxa_consensus_divergence_at_genus_returns_family_lca() {
        let taxon = found_taxon(run(
            vec![
                parsed("z1", 100.0, &lineage("zeta", "zeta-one")),
                parsed("b1", 100.0, &lineage("beta", "beta-one")),
                parsed("z2", 100.0, &lineage("zeta", "zeta-two")),
                parsed("a1", 100.0, &lineage("alpha", "alpha-one")),
            ],
            ConsensusStrategy::Cautious,
        ));

        assert_eq!(taxon.reached_rank, LinnaeanRank::Family);
        assert_eq!(taxon.identifier, "bacillaceae");
        assert_eq!(taxon.taxonomy.as_deref(), Some(FAMILY));
        assert!(!taxon.single_match);
        assert_eq!(
            bean_summary(&taxon),
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
        let beans = taxon.consensus_beans.unwrap();
        assert!(beans.iter().all(|b| b.rank == LinnaeanRank::Genus));
        assert_eq!(
            beans[1].taxonomy.as_deref(),
            Some(lineage("alpha", "alpha-one").as_str())
        );
    }

    // FR-MULTI-05: the max identity of the diverging level is only used for
    // rank adjustment; the reported `perc_identity` comes from the reference
    // record (Cautious → lowest identity among the shortest lineages).
    // Catalog §3.11 states "`percIdentity` = max": the code does not do that.
    #[test]
    fn find_multi_taxa_consensus_lca_perc_identity_comes_from_reference_record()
    {
        let taxon = found_taxon(run(
            vec![
                parsed("a", 100.0, &lineage("alpha", "alpha-one")),
                parsed("b", 99.5, &lineage("beta", "beta-one")),
            ],
            ConsensusStrategy::Cautious,
        ));

        assert_eq!(taxon.reached_rank, LinnaeanRank::Family);
        assert_eq!(taxon.perc_identity, 99.5);
    }

    // BUG-R03: divergence at the first rank has no common ancestor.
    #[test]
    fn find_multi_taxa_consensus_divergence_at_domain_returns_no_consensus() {
        match run(
            vec![
                parsed("a", 100.0, "d__bacteria;p__bacillota"),
                parsed("b", 100.0, "d__archaea;p__euryarchaeota"),
            ],
            ConsensusStrategy::Cautious,
        ) {
            ConsensusResult::NoConsensusFound(missing) => {
                assert_eq!(missing.query, "q1")
            }
            other => panic!("expected NoConsensusFound, got {other:?}"),
        }
    }

    /// A: d..f (5 ranks); B and C: d..s (7 ranks), same genus, different
    /// species.
    fn mixed_depth_records() -> Vec<BlastResultRow> {
        vec![
            parsed("C", 100.0, &lineage("bacillus", "bacillus-two")),
            parsed("A", 100.0, FAMILY),
            parsed("B", 100.0, &lineage("bacillus", "bacillus-one")),
        ]
    }

    // FR-MULTI-02 (Cautious → shortest lineage)
    #[test]
    fn find_multi_taxa_consensus_cautious_mixed_depth_stops_at_shortest_lineage(
    ) {
        let taxon = found_taxon(run(
            mixed_depth_records(),
            ConsensusStrategy::Cautious,
        ));

        assert_eq!(taxon.reached_rank, LinnaeanRank::Family);
        assert_eq!(taxon.identifier, "bacillaceae");
        assert_eq!(taxon.taxonomy.as_deref(), Some(FAMILY));
        assert_eq!(
            bean_summary(&taxon),
            vec![(
                "bacillaceae".to_string(),
                3,
                vec!["A".to_string(), "B".to_string(), "C".to_string()]
            )]
        );
    }

    // AC-05, FR-MULTI-02..05: Relaxed should walk the longest lineage; B and C
    // agree at genus and diverge at species → genus.
    // BUG-R06/BUG-R22: Relaxed evaluates ranks past the shortest lineage and
    // the agreeing levels no longer report the unchecked reference species.
    #[test]
    fn find_multi_taxa_consensus_relaxed_mixed_depth_returns_genus_lca() {
        let taxon =
            found_taxon(run(mixed_depth_records(), ConsensusStrategy::Relaxed));

        assert_eq!(taxon.reached_rank, LinnaeanRank::Genus);
        assert_eq!(taxon.identifier, "bacillus");
        assert_eq!(
            taxon.taxonomy.as_deref(),
            Some(format!("{FAMILY};g__bacillus").as_str())
        );
        assert_eq!(
            bean_summary(&taxon),
            vec![
                ("bacillus-one".to_string(), 1, vec!["B".to_string()]),
                ("bacillus-two".to_string(), 1, vec!["C".to_string()]),
            ]
        );
    }

    // FR-MULTI-01: ties in lineage length are broken by perc_identity, then
    // align_length, then subject_accession (ascending). The order is visible
    // in the folded bean accessions and in the reference record identity.
    #[test]
    fn find_multi_taxa_consensus_equal_length_records_sorted_by_tie_breakers() {
        let subtilis = lineage("bacillus", "bacillus-subtilis");
        let mut short_align = parsed("b", 100.0, &subtilis);
        short_align.align_length = 400;

        let records = vec![
            parsed("d", 100.0, &subtilis),
            short_align,
            parsed("a", 100.0, &subtilis),
            parsed("c", 99.5, &subtilis),
        ];

        let cautious =
            found_taxon(run(records.clone(), ConsensusStrategy::Cautious));
        assert_eq!(
            bean_summary(&cautious)[0].2,
            vec!["c", "b", "a", "d"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
        assert_eq!(cautious.perc_identity, 99.5);

        let relaxed = found_taxon(run(records, ConsensusStrategy::Relaxed));
        assert_eq!(relaxed.perc_identity, 100.0);
    }

    // FR-MULTI-06: the final taxon is exactly what
    // `build_blast_consensus_identity` builds for the LCA level.
    #[test]
    fn find_multi_taxa_consensus_lca_delegates_to_build_blast_consensus_identity(
    ) {
        let a = parsed("a", 100.0, &lineage("alpha", "alpha-one"));
        let b = parsed("b", 100.0, &lineage("beta", "beta-one"));
        let taxon = found_taxon(run(
            vec![b.clone(), a.clone()],
            ConsensusStrategy::Cautious,
        ));

        let reference = force_parsed_taxonomy(a.taxonomy.clone());
        let beans = [&a, &b]
            .iter()
            .map(|record| {
                let lineage = force_parsed_taxonomy(record.taxonomy.clone());
                ConsensusBean::from_taxonomy_bean(
                    lineage[5].clone(),
                    Some(record.subject_accession.clone()),
                    Taxonomy::taxonomy_beans_to_string(lineage),
                )
            })
            .collect::<Vec<_>>();
        let expected = build_blast_consensus_identity(
            "q1".to_string(),
            reference[4].clone(),
            100.0,
            4,
            reference.clone(),
            InterpolatedIdentity::new(
                Taxon::Bacteria,
                reference.iter().map(|b| b.reached_rank.clone()).collect(),
                None,
            )
            .unwrap(),
            Some(beans),
        );

        match expected {
            ConsensusResult::ConsensusFound(found) => {
                assert_eq!(Some(taxon), found.taxon)
            }
            other => panic!("expected ConsensusFound, got {other:?}"),
        }
    }

    #[test]
    #[should_panic(expected = "Invalid format taxonomic field.")]
    fn find_multi_taxa_consensus_literal_taxonomy_panics() {
        run(
            vec![
                row("a", 1, 100.0, 500, FAMILY),
                row("b", 1, 100.0, 500, FAMILY),
            ],
            ConsensusStrategy::Cautious,
        );
    }

    // NaN identity satisfies no cutoff, so no consensus is found (the
    // initial taxon that panicked on NaN was removed with BUG-R03).
    #[test]
    fn find_multi_taxa_consensus_nan_identity_returns_no_consensus() {
        match run(
            vec![parsed("a", f64::NAN, FAMILY), parsed("b", f64::NAN, FAMILY)],
            ConsensusStrategy::Cautious,
        ) {
            ConsensusResult::NoConsensusFound(missing) => {
                assert_eq!(missing.query, "q1")
            }
            other => panic!("expected NoConsensusFound, got {other:?}"),
        }
    }
}
