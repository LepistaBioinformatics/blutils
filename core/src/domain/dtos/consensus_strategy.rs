use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, clap::ValueEnum)]
pub enum ConsensusStrategy {
    /// Select the shortest taxonomic path to find consensus from.
    Cautious,

    /// Select the longest taxonomic path to find consensus from.
    Relaxed,
}
