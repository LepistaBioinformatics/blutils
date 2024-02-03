use super::{filter_rank_by_identity, get_taxonomy_from_position};
use crate::domain::dtos::{
    blast_builder::Taxon,
    blast_result::{
        BlastQueryConsensusResult, TaxonomyElement, ValidTaxonomicRanksEnum,
    },
};

pub(super) fn build_blast_consensus_identity(
    query: String,
    taxon: Taxon,
    preceding_element: Option<TaxonomyElement>,
    mut element: TaxonomyElement,
    descendent_element: Option<TaxonomyElement>,
    taxonomy: Vec<TaxonomyElement>,
) -> BlastQueryConsensusResult {
    element.rank = match filter_rank_by_identity(
        taxon.to_owned(),
        element.perc_identity,
        match preceding_element {
            None => None,
            Some(item) => Some(item.rank),
        },
        element.rank.to_owned(),
        match descendent_element {
            None => None,
            Some(item) => Some(item.rank),
        },
    ) {
        Err(err) => panic!("{err}"),
        Ok(res) => res,
    };

    let ordered_taxonomies = ValidTaxonomicRanksEnum::ordered_iter(Some(true));

    let updated_taxid = taxonomy.to_owned().iter().find_map(|i| {
        if i.rank == element.rank {
            Some(i.taxid)
        } else {
            None
        }
    });

    match updated_taxid {
        Some(taxid) => {
            let desired_rank_position = taxonomy
                .to_owned()
                .into_iter()
                .position(|item| item.taxid == taxid);

            let filtered_taxonomy = get_taxonomy_from_position(
                desired_rank_position.unwrap(),
                taxonomy.to_owned(),
            );

            element.mutated = true;
            element.taxid = taxid;
            element.taxonomy = Some(
                filtered_taxonomy
                    .into_iter()
                    .map(|i| i.taxonomy_to_string())
                    .collect::<Vec<String>>()
                    .join(";"),
            );
        }
        None => {
            let desired_rank_position = ordered_taxonomies
                .to_owned()
                .position(|rank| rank == &element.rank);

            let filtered_taxonomy = get_taxonomy_from_position(
                desired_rank_position.unwrap(),
                taxonomy.to_owned(),
            );

            let lower_taxonomy = filtered_taxonomy.last();

            if lower_taxonomy.is_some() {
                element.mutated = true;
                element.taxid = lower_taxonomy.unwrap().taxid;
                element.taxonomy = Some(
                    filtered_taxonomy
                        .into_iter()
                        .map(|i| i.taxonomy_to_string())
                        .collect::<Vec<String>>()
                        .join(";"),
                );
            }
        }
    }

    BlastQueryConsensusResult {
        query,
        taxon: Some(element),
    }
}
