use md5;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

// ? --------------------------------------------------------------------------
// ? Wrapper for Query Sequences
// ? --------------------------------------------------------------------------

#[derive(Debug)]
pub struct QuerySequence {
    pub hash_header: md5::Digest,
    pub full_identifier: &'static str,
    pub sequence: &'static str,
}

impl QuerySequence {
    /// This is the constructor like method for the `QuerySequenceDTO` object.
    #[allow(dead_code)]
    pub fn create(
        header: &'static str,
        sequence: &'static str,
    ) -> QuerySequence {
        let sequence_hash = md5::compute(sequence);

        QuerySequence {
            hash_header: sequence_hash,
            full_identifier: header,
            sequence,
        }
    }
}

// ? --------------------------------------------------------------------------
// ? Wrapper for Blast Builder
// ? --------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "camelCase")]
pub enum Taxon {
    Fungi,
    Bacteria,
    Eukaryotes,
}

impl FromStr for Taxon {
    type Err = ();

    fn from_str(input: &str) -> Result<Taxon, Self::Err> {
        match input {
            "f" | "Fungi" | "fungi" => Ok(Taxon::Fungi),
            "b" | "Bacteria" | "bacteria" => Ok(Taxon::Bacteria),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlastBuilder {
    // ? IO related parameters
    pub subject_reads: String,
    pub taxon: Taxon,

    // ? BlastN configuration related parameters
    pub out_format: &'static str,
    pub max_target_seqs: &'static str,
    pub perc_identity: &'static str,
    pub query_cov: &'static str,
    pub strand: &'static str,
    pub e_value: &'static str,
    pub min_consensus: &'static str,
}

impl BlastBuilder {
    pub fn create(subject_reads: &str, taxon: Taxon) -> Self {
        BlastBuilder {
            subject_reads: subject_reads.to_string(),
            taxon,
            out_format: "6",
            max_target_seqs: "100",
            perc_identity: "0.8",
            query_cov: "0.8",
            strand: "both",
            e_value: "0.001",
            min_consensus: "0.51",
        }
    }
}
