use super::{
    blast_builder::BlastBuilder, consensus_result::QueryWithConsensus,
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BlutilsOutput {
    pub(crate) results: Vec<QueryWithConsensus>,
    pub(crate) config: Option<BlastBuilder>,
}
