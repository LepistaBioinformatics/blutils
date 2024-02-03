use serde::Serialize;

use super::taxonomy::TaxonomyBean;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueryWithConsensusResult {
    pub query: String,
    pub taxon: Option<TaxonomyBean>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryWithoutConsensusResult {
    pub query: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ConsensusResult {
    /// No consensus option
    ///
    /// This option should be used when the consensus checking process not found
    /// an appropriate taxonomy.
    NoConsensusFound(QueryWithoutConsensusResult),

    /// Consensus option
    ///
    /// This option should be used when the consensus checking process found an
    /// appropriate taxonomy.
    ConsensusFound(QueryWithConsensusResult),
}
