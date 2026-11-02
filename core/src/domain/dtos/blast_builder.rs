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
    // Outputs written before these fields existed (e.g. blutils 7.1.3) lack
    // them, so they are defaulted on read (BUG-R04).
    #[serde(default = "default_is_config")]
    pub(crate) is_config: bool,
    #[serde(default = "Uuid::new_v4")]
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

fn default_is_config() -> bool {
    true
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn base() -> BlastBuilder {
        BlastBuilder::default("db", Taxon::Bacteria)
    }

    fn changed_keys(
        before: &BlastBuilder,
        after: &BlastBuilder,
    ) -> Vec<String> {
        let before = serde_json::to_value(before).unwrap();
        let after = serde_json::to_value(after).unwrap();
        let (Value::Object(before), Value::Object(after)) = (before, after)
        else {
            panic!("BlastBuilder must serialize as an object");
        };
        let mut keys: Vec<String> = before
            .iter()
            .filter(|(key, value)| after.get(*key) != Some(*value))
            .map(|(key, _)| key.clone())
            .collect();
        keys.sort();
        keys
    }

    #[test]
    fn blast_builder_default_sets_documented_values() {
        let builder = base();

        assert!(builder.is_config);
        assert_eq!(builder.blutils_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(builder.subject_reads, "db");
        assert!(matches!(builder.taxon, Taxon::Bacteria));
        assert_eq!(
            builder.out_format,
            "6 qseqid saccver staxid pident length mismatch gapopen qstart qend sstart send evalue bitscore"
        );
        assert_eq!(builder.max_target_seqs, 10);
        assert_eq!(builder.perc_identity, 80);
        assert_eq!(builder.query_cov, 80);
        assert!(matches!(builder.strand, Strand::Both));
        assert_eq!(builder.e_value, 0.001_f32);
        assert_eq!(builder.word_size, 15);
    }

    #[test]
    fn blast_builder_default_out_format_has_thirteen_fields_after_format_code()
    {
        let builder = base();
        let tokens: Vec<&str> = builder.out_format.split_whitespace().collect();

        assert_eq!(tokens[0], "6");
        assert_eq!(tokens[1..].len(), 13);
    }

    #[test]
    fn blast_builder_default_generates_distinct_run_ids() {
        assert_ne!(base().run_id, base().run_id);
    }

    #[test]
    fn blast_builder_with_max_target_seqs_changes_only_that_field() {
        let builder = base();
        let changed = builder.clone().with_max_target_seqs(5);
        assert_eq!(changed.max_target_seqs, 5);
        assert_eq!(changed_keys(&builder, &changed), vec!["maxTargetSeqs"]);
    }

    #[test]
    fn blast_builder_with_perc_identity_changes_only_that_field() {
        let builder = base();
        let changed = builder.clone().with_perc_identity(90);
        assert_eq!(changed.perc_identity, 90);
        assert_eq!(changed_keys(&builder, &changed), vec!["percIdentity"]);
    }

    #[test]
    fn blast_builder_with_query_cov_changes_only_that_field() {
        let builder = base();
        let changed = builder.clone().with_query_cov(70);
        assert_eq!(changed.query_cov, 70);
        assert_eq!(changed_keys(&builder, &changed), vec!["queryCov"]);
    }

    #[test]
    fn blast_builder_with_strand_changes_only_that_field() {
        let builder = base();
        let changed = builder.clone().with_strand(Strand::Plus);
        assert!(matches!(changed.strand, Strand::Plus));
        assert_eq!(changed_keys(&builder, &changed), vec!["strand"]);
    }

    #[test]
    fn blast_builder_with_e_value_changes_only_that_field() {
        let builder = base();
        let changed = builder.clone().with_e_value(0.5);
        assert_eq!(changed.e_value, 0.5_f32);
        assert_eq!(changed_keys(&builder, &changed), vec!["eValue"]);
    }

    #[test]
    fn blast_builder_with_word_size_changes_only_that_field() {
        let builder = base();
        let changed = builder.clone().with_word_size(11);
        assert_eq!(changed.word_size, 11);
        assert_eq!(changed_keys(&builder, &changed), vec!["wordSize"]);
    }

    #[test]
    fn blast_builder_serialize_uses_camel_case_keys() {
        let value = serde_json::to_value(base()).unwrap();
        let mut keys: Vec<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort();

        assert_eq!(
            keys,
            vec![
                "blutilsVersion",
                "eValue",
                "isConfig",
                "maxTargetSeqs",
                "outFormat",
                "percIdentity",
                "queryCov",
                "runId",
                "strand",
                "subjectReads",
                "taxon",
                "wordSize",
            ]
        );
        assert_eq!(value["strand"], "both");
        assert_eq!(value["taxon"], "bacteria");
    }

    // BUG-R04: legacy configs without `isConfig`/`runId` must deserialize.
    #[test]
    fn blast_builder_deserialize_without_is_config_and_run_id_uses_defaults() {
        let mut value = serde_json::to_value(base()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("isConfig");
        object.remove("runId");

        let builder: BlastBuilder = serde_json::from_value(value).unwrap();

        assert!(builder.is_config);
        assert!(!builder.run_id.is_nil());
    }

    #[test]
    fn strand_display_returns_lowercase_names() {
        assert_eq!(Strand::Both.to_string(), "both");
        assert_eq!(Strand::Plus.to_string(), "plus");
        assert_eq!(Strand::Minus.to_string(), "minus");
    }
}
