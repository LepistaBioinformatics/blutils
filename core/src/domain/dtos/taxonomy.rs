use super::linnaean_ranks::LinnaeanRank;

use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConsensusBean {
    pub rank: LinnaeanRank,
    pub identifier: String,
    pub occurrences: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaxonomyBean {
    pub rank: LinnaeanRank,
    pub identifier: String,
    pub perc_identity: f64,
    pub bit_score: f64,
    pub align_length: i64,
    pub mismatches: i64,
    pub gap_openings: i64,
    pub q_start: i64,
    pub q_end: i64,
    pub s_start: i64,
    pub s_end: i64,
    pub e_value: f64,
    pub taxonomy: Option<String>,
    pub mutated: bool,
    pub consensus_beans: Option<Vec<ConsensusBean>>,
}

impl TaxonomyBean {
    pub fn taxonomy_to_string(&self) -> String {
        format!("{}__{}", self.rank.to_string(), self.identifier.to_string())
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Taxonomy {
    Literal(String),
    Parsed(Vec<TaxonomyBean>),
}
