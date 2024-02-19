use crate::domain::dtos::{
    consensus_result::{ConsensusBean, QueryWithConsensus},
    linnaean_ranks::{
        InterpolatedIdentity, LinnaeanRank, RankedLinnaeanIdentity::*,
    },
    taxonomy_bean::TaxonomyBean,
};

pub(super) fn build_blast_consensus_identity(
    query: String,
    mut bean: TaxonomyBean,
    max_allowed_identity: f64,
    target_as_single_match: bool,
    bean_index: usize,
    taxonomy: Vec<TaxonomyBean>,
    interpolated_taxonomy: InterpolatedIdentity,
    consensus_beans: Option<Vec<ConsensusBean>>,
) -> QueryWithConsensus {
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
    // Check for mutation of record
    //
    if let Some(allowed_rank) = bean.max_allowed_rank.to_owned() {
        bean.mutated = bean.reached_rank != allowed_rank;
    }

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

        bean.consensus_beans = Some(consensus_beans.to_owned());
    }

    if let Some(_bean) = taxonomy.get(bean_index) {
        let adjusted_taxonomy: Vec<TaxonomyBean> = {
            let base_iterator = interpolated_taxonomy
                .get_adjusted_taxonomy_by_identity(
                    max_allowed_identity,
                    taxonomy.to_owned(),
                )
                .into_iter();

            if target_as_single_match && consensus_beans.len() == 1 {
                base_iterator.collect()
            } else {
                base_iterator
                    .enumerate()
                    .take_while(|(index, _)| index <= &bean_index)
                    .map(|i| i.1.to_owned())
                    .collect()
            }
        };

        let last_taxonomy = adjusted_taxonomy.last().unwrap_or(_bean);

        bean.identifier = last_taxonomy.identifier.to_owned();
        bean.reached_rank = last_taxonomy.reached_rank.to_owned();
        bean.taxonomy = Some(
            adjusted_taxonomy
                .into_iter()
                .map(|i| i.taxonomy_to_string())
                .collect::<Vec<String>>()
                .join(";"),
        )
        //bean.taxonomy = Some(
        //    filtered_taxonomy
        //        .into_iter()
        //        .map(|i| i.taxonomy_to_string())
        //        .collect::<Vec<String>>()
        //        .join(";"),
        //);
    } else {
        panic!("No taxonomy found for bean at index: {}", bean_index);
    }

    /* match taxonomy.get(bean_index) {
        Some(_bean) => {
            let desired_rank_position =
                taxonomy.to_owned().into_iter().position(|item| {
                    item.identifier == _bean.identifier.to_owned()
                });

            let filtered_taxonomy = get_taxonomy_from_position(
                desired_rank_position.unwrap(),
                taxonomy.to_owned(),
            );

            println!("Filtered taxonomy: {:?}", filtered_taxonomy);

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
                        rank == bean.reached_rank
                    }
                    RankedLinnaeanIdentity::NonDefaultRank(rank, _) => {
                        LinnaeanRank::Other(rank) == bean.reached_rank
                    }
                });

            let filtered_taxonomy = get_taxonomy_from_position(
                desired_rank_position.unwrap(),
                taxonomy.to_owned(),
            );

            let lower_taxonomy = filtered_taxonomy.last();

            if lower_taxonomy.is_some() {
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
    } */

    QueryWithConsensus {
        query,
        taxon: Some(bean),
    }
}
