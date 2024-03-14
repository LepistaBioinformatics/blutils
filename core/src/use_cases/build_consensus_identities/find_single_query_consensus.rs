use super::{find_multi_taxa_consensus, force_parsed_taxonomy};
use crate::domain::dtos::{
    blast_builder::Taxon,
    blast_result::BlastResultRow,
    consensus_result::{
        ConsensusBean, ConsensusResult, QueryWithConsensus,
        QueryWithoutConsensus,
    },
    consensus_strategy::ConsensusStrategy,
    linnaean_ranks::InterpolatedIdentity,
    taxonomy_bean::{Taxonomy, TaxonomyBean},
};

use mycelium_base::utils::errors::MappedErrors;
use std::collections::HashMap;

pub(super) fn find_single_query_consensus(
    query: String,
    result: Vec<BlastResultRow>,
    taxon: Taxon,
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
        let bit_score_matches = result
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
        if bit_score_matches.len() == 0 {
            return Ok(ConsensusResult::NoConsensusFound(no_consensus));
        }
        //
        // Fetch the lower taxonomic rank case only one record returned.
        //
        if bit_score_matches.len() == 1 {
            //
            // Unwrap the single match record.
            //
            let target_blast_match = match bit_score_matches.first() {
                Some(record) => record,
                None => {
                    return Ok(ConsensusResult::NoConsensusFound(no_consensus));
                }
            };
            //
            // This action prevents the program to panic when the taxonomy
            // is not already parsed.
            //
            let taxonomies =
                force_parsed_taxonomy(target_blast_match.taxonomy.to_owned());
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
            // Unwrap the last taxonomy element to be used as the final taxon.
            //
            let target_bean = match identity_adjusted_taxonomy.last() {
                Some(bean) => bean.to_owned(),
                None => panic!(
                    "No taxonomy found for result: {:?}",
                    target_blast_match.subject_accession
                ),
            };
            //
            // Initialize the consensus bean.
            //
            let consensus_bean = ConsensusBean::from_taxonomy_bean(
                target_bean.to_owned(),
                Some(target_blast_match.subject_accession.to_owned()),
                Taxonomy::taxonomy_beans_to_string(taxonomies.to_owned()),
            );
            //
            // Return the consensus result.
            //
            return Ok(ConsensusResult::ConsensusFound(QueryWithConsensus {
                query,
                taxon: Some(TaxonomyBean {
                    single_match: true,
                    identifier: target_bean.identifier.to_owned(),
                    taxonomy: Some(
                        identity_adjusted_taxonomy
                            .into_iter()
                            .map(|i| i.taxonomy_to_string())
                            .collect::<Vec<String>>()
                            .join(";"),
                    ),
                    consensus_beans: Some(ConsensusBean::fold_consensus_list(
                        vec![consensus_bean],
                    )),
                    ..target_bean
                }),
                run_id: None,
            }));
        }
        //
        // Fetch the lower taxonomic rank case more than one record returned.
        //
        if bit_score_matches.len() > 1 {
            match find_multi_taxa_consensus(
                bit_score_matches,
                taxon,
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
