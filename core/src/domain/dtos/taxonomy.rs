use super::linnaean_ranks::LinnaeanRanks;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaxonomyBean {
    pub rank: LinnaeanRanks,
    pub taxid: i64,
    pub perc_identity: f64,
    pub taxonomy: Option<String>,
    pub mutated: bool,
}

impl TaxonomyBean {
    pub fn taxonomy_to_string(&self) -> String {
        format!("{}__{}", self.rank.to_string(), self.taxid.to_string())
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Taxonomy {
    Literal(String),
    Parsed(Vec<TaxonomyBean>),
}
