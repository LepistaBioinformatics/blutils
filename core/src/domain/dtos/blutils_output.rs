use super::{
    blast_builder::BlastBuilder, consensus_result::QueryWithConsensus,
};

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BlutilsOutput {
    pub(crate) results: Vec<QueryWithConsensus>,
    pub(crate) config: Option<BlastBuilder>,
}
