use std::collections::HashMap;

use super::get_taxonomy_from_position;
use crate::domain::dtos::{
    consensus_result::QueryWithConsensus,
    linnaean_ranks::{
        InterpolatedIdentity, LinnaeanRank, RankedLinnaeanIdentity,
    },
    taxonomy::{ConsensusBean, TaxonomyBean},
};

pub(super) fn build_blast_consensus_identity(
    query: String,
    mut bean: TaxonomyBean,
    bean_index: usize,
    taxonomy: Vec<TaxonomyBean>,
    interpolated_taxonomy: InterpolatedIdentity,
    consensus_beans: Option<Vec<TaxonomyBean>>,
) -> QueryWithConsensus {
    //
    // Update the rank of the bean according to the interpolated taxonomy.
    //
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

    //
    // Fold consensus beans by rank and identity to aggregate the occurrences
    // of each taxon.
    //
    let mut consensus_beans = consensus_beans
        .unwrap_or_default()
        .into_iter()
        .fold(HashMap::<String, ConsensusBean>::new(), |mut acc, bean| {
            let rank = bean.rank.to_string();
            let identifier = bean.identifier.to_string();

            let consensus_bean = acc
                .entry(format!("{}__{}", rank, identifier))
                .or_insert(ConsensusBean {
                    rank: bean.rank,
                    identifier: bean.identifier,
                    occurrences: 0,
                });

            consensus_bean.occurrences += 1;

            acc
        })
        .into_iter()
        .map(|(_, bean)| bean)
        .collect::<Vec<ConsensusBean>>();

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

    match taxonomy.get(bean_index) {
        Some(_bean) => {
            let desired_rank_position =
                taxonomy.to_owned().into_iter().position(|item| {
                    item.identifier == _bean.identifier.to_owned()
                });

            let filtered_taxonomy = get_taxonomy_from_position(
                desired_rank_position.unwrap(),
                taxonomy.to_owned(),
            );

            bean.mutated = true;
            bean.identifier = _bean.identifier.to_owned();
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
                bean.identifier = lower_taxonomy.unwrap().identifier.to_owned();
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
