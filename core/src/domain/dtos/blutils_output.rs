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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::taxon::Taxon;

    #[test]
    fn blutils_output_default_serializes_empty_results_and_null_config() {
        assert_eq!(
            serde_json::to_string(&BlutilsOutput::default()).unwrap(),
            r#"{"results":[],"config":null}"#
        );
    }

    #[test]
    fn blutils_output_with_config_round_trips() {
        let output = BlutilsOutput {
            results: vec![QueryWithConsensus {
                run_id: None,
                query: "q1".to_string(),
                taxon: None,
            }],
            config: Some(BlastBuilder::default("db", Taxon::Fungi)),
        };

        let json = serde_json::to_value(&output).unwrap();
        let back: BlutilsOutput = serde_json::from_value(json.clone()).unwrap();

        assert_eq!(json["results"][0]["runId"], serde_json::Value::Null);
        assert_eq!(json["config"]["isConfig"], true);
        assert_eq!(serde_json::to_value(&back).unwrap(), json);
    }
}
