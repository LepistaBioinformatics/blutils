use super::{
    blast_builder::BlastBuilder, consensus_result::QueryWithConsensus,
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlutilsOutput {
    pub results: Vec<QueryWithConsensus>,
    pub config: Option<BlastBuilder>,
}

// Implements Default for BlutilsOutput
impl Default for BlutilsOutput {
    fn default() -> Self {
        BlutilsOutput {
            results: Vec::new(),
            config: None,
        }
    }
}
