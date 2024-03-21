use super::{build_blast_consensus_identity, force_parsed_taxonomy};
use crate::domain::dtos::{
    blast_result::BlastResultRow,
    consensus_result::{
        ConsensusBean, ConsensusResult, QueryWithConsensus,
        QueryWithoutConsensus,
    },
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

    let lowest_taxonomy_of_higher_rank = {
        let mut rank_taxonomies = sorted_taxonomies.first().unwrap().to_owned();

        rank_taxonomies.sort_by(|a, b| {
            a.perc_identity.partial_cmp(&b.perc_identity).unwrap()
        });

        rank_taxonomies.first().unwrap().to_owned()
    };

    // ? -----------------------------------------------------------------------
    // ? Initialize the final response based on high taxonomic rank
    // ? -----------------------------------------------------------------------

    let mut final_taxon = QueryWithConsensus {
        query: no_consensus_option.query.to_owned(),
        taxon: Some(lowest_taxonomy_of_higher_rank),
        run_id: None,
    };

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
        let level_max_taxonomy = sorted_taxonomies
            .iter()
            .zip(sorted_records.iter())
            .take_while(|(taxonomy, _)| index < taxonomy.len());

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
            let target_index = index - 1;
            let max_pert_identity = level_max_taxonomy
                .clone()
                .map(|(_, i)| i.perc_identity)
                .fold(0.0, |acc, i| if i > acc { i } else { acc });

            //
            // Build the consensus identity based on the multi-level taxonomy.
            //
            final_taxon = build_blast_consensus_identity(
                no_consensus_option.query.to_owned(),
                reference_taxonomy[target_index].to_owned(),
                max_pert_identity,
                false,
                target_index,
                reference_taxonomy.to_owned(),
                interpolated_identities.to_owned(),
                Some(consensus_beans),
            );

            break;
        }

        final_taxon = build_blast_consensus_identity(
            no_consensus_option.query.to_owned(),
            ref_taxonomy.to_owned(),
            ref_taxonomy.perc_identity,
            true,
            index,
            reference_taxonomy.to_owned(),
            interpolated_identities.to_owned(),
            Some(consensus_beans),
        )
    }

    Ok(ConsensusResult::ConsensusFound(final_taxon))
}
