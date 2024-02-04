use super::{
    find_multi_taxa_consensus, force_parsed_taxonomy,
    get_taxonomy_from_position,
};
use crate::domain::dtos::{
    blast_builder::BlastBuilder,
    blast_result::BlastResultRow,
    consensus_result::{
        ConsensusResult, QueryWithConsensus, QueryWithoutConsensus,
    },
    consensus_strategy::ConsensusStrategy,
    linnaean_ranks::{
        InterpolatedIdentity, LinnaeanRank, RankedLinnaeanIdentity,
    },
};

use mycelium_base::utils::errors::MappedErrors;
use std::collections::HashMap;

pub(super) fn find_single_query_consensus(
    query: String,
    result: Vec<BlastResultRow>,
    config: BlastBuilder,
    strategy: ConsensusStrategy,
) -> Result<ConsensusResult, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Group results by bit-score
    // ? -----------------------------------------------------------------------

    let grouped_results = result.to_owned().into_iter().fold(
        HashMap::<i64, Vec<BlastResultRow>>::new(),
        |mut init, result| {
            init.entry(result.bit_score)
                .or_insert_with(Vec::new)
                .push(result);

            init
        },
    );

    // ? -----------------------------------------------------------------------
    // ? Evaluate results by bit-score
    // ? -----------------------------------------------------------------------

    let mut sorted_keys = grouped_results.keys().cloned().collect::<Vec<i64>>();
    sorted_keys.sort_by(|a, b| b.cmp(a));

    let no_consensus = QueryWithoutConsensus {
        query: query.to_owned(),
    };

    for score in sorted_keys.to_owned().into_iter() {
        let score_results = result
            .to_owned()
            .into_iter()
            .filter_map(|i| {
                if i.bit_score != score {
                    None
                } else {
                    match i.to_owned().parse_taxonomy() {
                        Err(err) => panic!("{err}"),
                        Ok(res) => Some(res),
                    }
                }
            })
            .collect::<Vec<BlastResultRow>>();
        //
        // Early return case no results found.
        //
        if score_results.len() == 0 {
            return Ok(ConsensusResult::NoConsensusFound(no_consensus));
        }
        //
        // Fetch the lower taxonomic rank case only one record returned.
        //
        if score_results.len() == 1 {
            //
            // This action prevents the program to panic when the taxonomy
            // is not already parsed.
            //
            let taxonomies =
                force_parsed_taxonomy(score_results[0].taxonomy.to_owned());
            //
            // Generate interpolated identities for the taxon.
            //
            let interpolated_identities = InterpolatedIdentity::new(
                config.to_owned().taxon.to_owned(),
                taxonomies
                    .clone()
                    .into_iter()
                    .map(|bean| bean.rank)
                    .collect(),
            )?;

            for rank in LinnaeanRank::ordered_iter(None) {
                //
                // Try to find the consensus for the rank.
                //
                match taxonomies
                    .to_owned()
                    .into_iter()
                    .find(|i| &i.rank == rank)
                {
                    None => {
                        return Ok(ConsensusResult::NoConsensusFound(
                            no_consensus,
                        ))
                    }
                    Some(mut res) => {
                        let identity_adjusted_rank =
                            match interpolated_identities
                                .get_rank_adjusted_by_identity(
                                    score_results[0].perc_identity,
                                ) {
                                Some(rank) => rank,
                                None => {
                                    return Ok(
                                        ConsensusResult::NoConsensusFound(
                                            no_consensus,
                                        ),
                                    )
                                }
                            };

                        let reviewed_rank = match identity_adjusted_rank {
                            RankedLinnaeanIdentity::DefaultRank(rank, _) => {
                                rank
                            }
                            RankedLinnaeanIdentity::NonDefaultRank(rank, _) => {
                                LinnaeanRank::Other(rank)
                            }
                        };

                        if res.to_owned().rank == reviewed_rank {
                            res.mutated = true;
                        }

                        let position = match interpolated_identities
                            .interpolation()
                            .to_owned()
                            .into_iter()
                            .position(|i| match i {
                                RankedLinnaeanIdentity::DefaultRank(
                                    rank,
                                    _,
                                ) => rank == reviewed_rank,
                                RankedLinnaeanIdentity::NonDefaultRank(
                                    rank,
                                    _,
                                ) => LinnaeanRank::Other(rank) == reviewed_rank,
                            }) {
                            Some(position) => position,
                            None => {
                                panic!("Unexpected error detected on find consensus position")
                            }
                        };

                        let filtered_taxonomy = get_taxonomy_from_position(
                            position,
                            taxonomies.to_owned(),
                        );

                        let lower_taxonomy = filtered_taxonomy.last();

                        if lower_taxonomy.is_some() {
                            let lower_taxonomy = lower_taxonomy.unwrap();

                            res.rank = lower_taxonomy.to_owned().rank;
                            res.taxid = lower_taxonomy.taxid;
                            res.taxonomy = Some(
                                filtered_taxonomy
                                    .into_iter()
                                    .map(|i| i.taxonomy_to_string())
                                    .collect::<Vec<String>>()
                                    .join(";"),
                            );
                        }

                        return Ok(ConsensusResult::ConsensusFound(
                            QueryWithConsensus {
                                query,
                                taxon: Some(res),
                            },
                        ));
                    }
                }
            }

            return Ok(ConsensusResult::NoConsensusFound(no_consensus));
        }
        //
        // Fetch the lower taxonomic rank case more than one record returned.
        //
        if score_results.len() > 1 {
            match find_multi_taxa_consensus(
                score_results,
                config.to_owned().taxon,
                no_consensus.clone(),
                strategy.to_owned(),
            ) {
                Err(err) => panic!("{err}"),
                Ok(res) => return Ok(res),
            };
        }
    }

    // Execute the default option
    //
    // If consensus identity not found in the previous steps, assumes by default
    // a no consensus option.
    Ok(ConsensusResult::NoConsensusFound(no_consensus))
}
