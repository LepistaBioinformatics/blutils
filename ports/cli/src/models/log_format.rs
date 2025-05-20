use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "camelCase")]
pub enum LogFormat {
    /// ANSI format
    ///
    /// This format is human-readable and colorful.
    Ansi,

    /// YAML format
    ///
    /// This format is machine-readable and can be used for log analysis.
    Jsonl,
}
