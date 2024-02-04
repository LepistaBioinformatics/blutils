use super::taxonomy::TaxonomyBean;

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueryWithConsensus {
    pub query: String,
    pub taxon: Option<TaxonomyBean>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueryWithoutConsensus {
    pub query: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ConsensusResult {
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
