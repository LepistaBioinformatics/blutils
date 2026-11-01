use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, clap::ValueEnum)]
pub enum ConsensusStrategy {
    /// Select the shortest taxonomic path to find consensus from.
    Cautious,

    /// Select the longest taxonomic path to find consensus from.
    Relaxed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consensus_strategy_serde_uses_pascal_case_variant_names() {
        for (strategy, name) in [
            (ConsensusStrategy::Cautious, "\"Cautious\""),
            (ConsensusStrategy::Relaxed, "\"Relaxed\""),
        ] {
            assert_eq!(serde_json::to_string(&strategy).unwrap(), name);
            let back: ConsensusStrategy = serde_json::from_str(name).unwrap();
            assert_eq!(serde_json::to_string(&back).unwrap(), name);
        }
    }

    #[test]
    fn consensus_strategy_deserialize_lowercase_fails() {
        assert!(
            serde_json::from_str::<ConsensusStrategy>("\"cautious\"").is_err()
        );
    }
}
