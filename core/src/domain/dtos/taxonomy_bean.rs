use super::{consensus_result::ConsensusBean, linnaean_ranks::LinnaeanRank};

use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxonomyBean {
    pub reached_rank: LinnaeanRank,
    pub max_allowed_rank: Option<LinnaeanRank>,
    pub identifier: String,
    pub perc_identity: f64,
    pub bit_score: f64,
    pub taxonomy: Option<String>,
    pub mutated: bool,
    pub single_match: bool,
    pub consensus_beans: Option<Vec<ConsensusBean>>,
}

impl TaxonomyBean {
    pub fn taxonomy_to_string(&self) -> String {
        format!(
            "{}__{}",
            self.reached_rank.to_string(),
            self.identifier.to_string()
        )
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Taxonomy {
    Literal(String),
    Parsed(Vec<TaxonomyBean>),
}

impl Taxonomy {
    pub(crate) fn taxonomy_beans_to_string(
        taxonomies: Vec<TaxonomyBean>,
    ) -> String {
        taxonomies
            .into_iter()
            .map(|i| i.taxonomy_to_string())
            .collect::<Vec<String>>()
            .join(";")
    }
}
