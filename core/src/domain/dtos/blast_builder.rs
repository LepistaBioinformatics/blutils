use super::taxon::Taxon;

use md5;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

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

#[derive(Clone, Debug, Serialize, clap::ValueEnum, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Strand {
    Both,
    Plus,
    Minus,
}

impl fmt::Display for Strand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Strand::Both => write!(f, "both"),
            Strand::Plus => write!(f, "plus"),
            Strand::Minus => write!(f, "minus"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlastBuilder {
    pub(crate) is_config: bool,
    pub(crate) run_id: Uuid,
    pub(crate) blutils_version: String,

    // ? IO related parameters
    pub subject_reads: String,
    pub taxon: Taxon,

    // ? BlastN configuration related parameters
    pub out_format: String,
    pub max_target_seqs: i32,
    pub perc_identity: i32,
    pub query_cov: i32,
    pub strand: Strand,
    pub e_value: f32,
    pub word_size: i32,
}

impl BlastBuilder {
    pub fn default(subject_reads: &str, taxon: Taxon) -> Self {
        BlastBuilder {
            is_config: true,
            run_id: Uuid::new_v4(),
            blutils_version: env!("CARGO_PKG_VERSION").to_string(),
            subject_reads: subject_reads.to_string(),
            taxon,
            out_format: "6 qseqid saccver staxid pident length mismatch gapopen qstart qend sstart send evalue bitscore".to_string(),
            max_target_seqs: 10,
            perc_identity: 80,
            query_cov: 80,
            strand: Strand::Both,
            e_value: 0.001,
            word_size: 15,
        }
    }

    pub fn with_max_target_seqs(mut self, max_target_seqs: i32) -> Self {
        self.max_target_seqs = max_target_seqs;
        self
    }

    pub fn with_perc_identity(mut self, perc_identity: i32) -> Self {
        self.perc_identity = perc_identity;
        self
    }

    pub fn with_query_cov(mut self, query_cov: i32) -> Self {
        self.query_cov = query_cov;
        self
    }

    pub fn with_strand(mut self, strand: Strand) -> Self {
        self.strand = strand;
        self
    }

    pub fn with_e_value(mut self, e_value: f32) -> Self {
        self.e_value = e_value;
        self
    }

    pub fn with_word_size(mut self, word_size: i32) -> Self {
        self.word_size = word_size;
        self
    }
}
