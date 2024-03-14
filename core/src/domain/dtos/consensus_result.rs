use super::{linnaean_ranks::LinnaeanRank, taxonomy_bean::TaxonomyBean};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryWithConsensus {
    pub run_id: Option<Uuid>,
    pub query: String,
    pub taxon: Option<TaxonomyBean>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryWithoutConsensus {
    pub query: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConsensusResult {
    /// No consensus option
    ///
    /// This option should be used when the consensus checking process not found
    /// an appropriate taxonomy.
    NoConsensusFound(QueryWithoutConsensus),

    /// Consensus option
    ///
    /// This option should be used when the consensus checking process found an
    /// appropriate taxonomy.
    ConsensusFound(QueryWithConsensus),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsensusBean {
    pub rank: LinnaeanRank,
    pub identifier: String,
    pub occurrences: i32,
    pub taxonomy: Option<String>,
    pub accessions: Vec<String>,
}

impl ConsensusBean {
    pub(crate) fn from_taxonomy_bean(
        bean: TaxonomyBean,
        accession: Option<String>,
        taxonomy: String,
    ) -> Self {
        ConsensusBean {
            rank: bean.reached_rank,
            identifier: bean.identifier,
            occurrences: 0,
            taxonomy: taxonomy.into(),
            accessions: match accession {
                Some(res) => vec![res],
                _ => vec![],
            },
        }
    }

    pub(crate) fn fold_consensus_list(consensus: Vec<Self>) -> Vec<Self> {
        consensus
            .iter()
            .fold(HashMap::<String, Self>::new(), |mut acc, bean| {
                let rank = bean.rank.to_string();
                let identifier = bean.identifier.to_string();

                let consensus_bean = acc
                    .entry(format!("{}__{}", rank, identifier))
                    .or_insert(Self {
                        occurrences: 0,
                        ..bean.clone()
                    });

                consensus_bean.accessions.extend(bean.accessions.to_owned());
                consensus_bean.accessions.dedup();
                consensus_bean.occurrences += 1;

                acc
            })
            .into_iter()
            .map(|(_, bean)| bean)
            .collect::<Vec<Self>>()
    }
}
