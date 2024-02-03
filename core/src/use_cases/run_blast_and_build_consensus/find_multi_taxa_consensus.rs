use super::{
    build_blast_consensus_identity, force_parsed_taxonomy,
    get_rank_lowest_statistics,
};
use crate::domain::dtos::{
    blast_builder::Taxon,
    blast_result::BlastResultRow,
    consensus_result::{
        ConsensusResult, QueryWithConsensusResult, QueryWithoutConsensusResult,
    },
    consensus_strategy::ConsensusStrategy,
    linnaean_ranks::LinnaeanRanks,
    taxonomy::TaxonomyBean,
};

use mycelium_base::utils::errors::MappedErrors;

/// Find the consensus among Blast results with multiple output.
///
/// In some cases blast results returns a list if records with the same percent
/// identity and bit-score. In this cases this logic could be applied to solve
/// the problem.
pub(super) fn find_multi_taxa_consensus(
    records: Vec<BlastResultRow>,
    taxon: Taxon,
    no_consensus_option: QueryWithoutConsensusResult,
    strategy: ConsensusStrategy,
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
        a_taxonomy.len().cmp(&b_taxonomy.len())
    });

    let reference_taxonomy = match strategy {
        ConsensusStrategy::Cautious => {
            sorted_records.first().unwrap().to_owned()
        }
        ConsensusStrategy::Relaxed => sorted_records.last().unwrap().to_owned(),
    };

    // ? -----------------------------------------------------------------------
    // ? Build the bi-dimensional
    //
    // Each position of the vector contain a vector of the `TaxonomyElement`
    // type.
    //
    // ? -----------------------------------------------------------------------

    let taxonomies = sorted_records
        .into_iter()
        .map(|i| force_parsed_taxonomy(i.taxonomy))
        .collect::<Vec<Vec<TaxonomyBean>>>();

    let lowest_taxonomy_of_higher_rank =
        get_rank_lowest_statistics(taxonomies.first().unwrap().to_owned());

    // ? -----------------------------------------------------------------------
    // ? Initialize the final response based on high taxonomic rank
    // ? -----------------------------------------------------------------------

    let mut final_taxon = QueryWithConsensusResult {
        query: no_consensus_option.query.to_owned(),
        taxon: Some(lowest_taxonomy_of_higher_rank),
    };

    // ? -----------------------------------------------------------------------
    // ? Try to update the final response
    // ? -----------------------------------------------------------------------

    let reference_taxonomy_elements =
        force_parsed_taxonomy(reference_taxonomy.taxonomy.to_owned());

    for (index, ref_taxonomy) in reference_taxonomy_elements.iter().enumerate()
    {
        let mut level_taxonomies = Vec::<(LinnaeanRanks, i64)>::new();

        for taxonomy in taxonomies.iter() {
            if index < taxonomy.len() {
                if !level_taxonomies.contains(&(
                    taxonomy[index].rank.to_owned(),
                    taxonomy[index].taxid,
                )) {
                    level_taxonomies.push((
                        taxonomy[index].rank.to_owned(),
                        taxonomy[index].taxid,
                    ));
                }
            }
        }

        let loop_reference_taxonomy_elements =
            reference_taxonomy_elements.to_owned();

        if level_taxonomies.len() > 1 {
            final_taxon = build_blast_consensus_identity(
                no_consensus_option.query.to_owned(),
                taxon.to_owned(),
                reference_taxonomy_elements[index - 1].to_owned(),
                loop_reference_taxonomy_elements.to_owned(),
            );

            break;
        }

        final_taxon = build_blast_consensus_identity(
            no_consensus_option.query.to_owned(),
            taxon.to_owned(),
            ref_taxonomy.to_owned(),
            loop_reference_taxonomy_elements,
        );
    }

    Ok(ConsensusResult::ConsensusFound(final_taxon))
}
