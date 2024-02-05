use super::get_taxonomy_from_position;
use crate::domain::dtos::{
    consensus_result::QueryWithConsensus,
    linnaean_ranks::{
        InterpolatedIdentity, LinnaeanRank, RankedLinnaeanIdentity,
    },
    taxonomy::TaxonomyBean,
};

pub(super) fn build_blast_consensus_identity(
    query: String,
    mut bean: TaxonomyBean,
    taxonomy: Vec<TaxonomyBean>,
    interpolated_taxonomy: InterpolatedIdentity,
) -> QueryWithConsensus {
    bean.rank = match interpolated_taxonomy
        .get_rank_adjusted_by_identity(bean.perc_identity as f64)
    {
        Some(identity_adjusted_rank) => match identity_adjusted_rank {
            RankedLinnaeanIdentity::DefaultRank(rank, _) => rank,
            RankedLinnaeanIdentity::NonDefaultRank(rank, _) => {
                LinnaeanRank::Other(rank)
            }
        },
        None => bean.rank,
    };

    let updated_taxid = taxonomy.to_owned().iter().find_map(|i| {
        if i.rank == bean.rank {
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

            bean.mutated = true;
            bean.taxid = taxid;
            bean.taxonomy = Some(
                filtered_taxonomy
                    .into_iter()
                    .map(|i| i.taxonomy_to_string())
                    .collect::<Vec<String>>()
                    .join(";"),
            );
        }
        None => {
            let desired_rank_position = interpolated_taxonomy
                .interpolation()
                .to_owned()
                .into_iter()
                .rev()
                .to_owned()
                .position(|i| match i {
                    RankedLinnaeanIdentity::DefaultRank(rank, _) => {
                        rank == bean.rank
                    }
                    RankedLinnaeanIdentity::NonDefaultRank(rank, _) => {
                        LinnaeanRank::Other(rank) == bean.rank
                    }
                });

            let filtered_taxonomy = get_taxonomy_from_position(
                desired_rank_position.unwrap(),
                taxonomy.to_owned(),
            );

            let lower_taxonomy = filtered_taxonomy.last();

            if lower_taxonomy.is_some() {
                bean.mutated = true;
                bean.taxid = lower_taxonomy.unwrap().taxid;
                bean.taxonomy = Some(
                    filtered_taxonomy
                        .into_iter()
                        .map(|i| i.taxonomy_to_string())
                        .collect::<Vec<String>>()
                        .join(";"),
                );
            }
        }
    }

    QueryWithConsensus {
        query,
        taxon: Some(bean),
    }
}
