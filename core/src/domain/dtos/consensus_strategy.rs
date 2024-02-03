use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, clap::ValueEnum)]
pub enum ConsensusStrategy {
    Cautious,
    Relaxed,
}
