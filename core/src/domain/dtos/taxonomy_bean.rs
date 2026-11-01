use super::{consensus_result::ConsensusBean, linnaean_ranks::LinnaeanRank};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::row;

    fn bean(rank: LinnaeanRank, identifier: &str) -> TaxonomyBean {
        TaxonomyBean {
            reached_rank: rank,
            max_allowed_rank: None,
            identifier: identifier.to_string(),
            perc_identity: 99.0,
            bit_score: 100.0,
            taxonomy: None,
            mutated: false,
            single_match: false,
            consensus_beans: None,
        }
    }

    fn parse(lineage: &str) -> Vec<TaxonomyBean> {
        let mut hit = row("NR_1.1", 2, 99.0, 100, lineage);
        match hit.parse_taxonomy().unwrap().taxonomy {
            Taxonomy::Parsed(beans) => beans,
            Taxonomy::Literal(_) => panic!("expected parsed taxonomy"),
        }
    }

    #[test]
    fn taxonomy_to_string_default_rank_uses_short_code() {
        assert_eq!(
            bean(LinnaeanRank::Species, "x").taxonomy_to_string(),
            "s__x"
        );
    }

    #[test]
    fn taxonomy_to_string_other_rank_uses_slug() {
        assert_eq!(
            bean(LinnaeanRank::Other("strain".to_string()), "x")
                .taxonomy_to_string(),
            "strain__x"
        );
    }

    #[test]
    fn taxonomy_beans_to_string_empty_returns_empty_string() {
        assert_eq!(Taxonomy::taxonomy_beans_to_string(vec![]), "");
    }

    #[test]
    fn taxonomy_beans_to_string_joins_with_semicolon() {
        assert_eq!(
            Taxonomy::taxonomy_beans_to_string(vec![
                bean(LinnaeanRank::Domain, "2"),
                bean(LinnaeanRank::Genus, "561"),
            ]),
            "d__2;g__561"
        );
    }

    #[test]
    fn taxonomy_beans_to_string_short_ranks_round_trip_with_parse() {
        let lineage = "d__2;p__1224;c__1236;o__91347;f__543;g__561;s__93973";

        assert_eq!(Taxonomy::taxonomy_beans_to_string(parse(lineage)), lineage);
    }

    #[test]
    fn taxonomy_beans_to_string_full_rank_names_normalize_to_short_codes() {
        assert_eq!(
            Taxonomy::taxonomy_beans_to_string(parse("domain__2;species__9")),
            "d__2;s__9"
        );
    }

    // BUG-R18: a short rank code deserializes to the default rank.
    #[test]
    fn taxonomy_bean_deserialize_short_rank_code_returns_species() {
        let json = r#"{"reachedRank":"s","maxAllowedRank":null,"identifier":"x","percIdentity":99.0,"bitScore":100.0,"taxonomy":null,"mutated":false,"singleMatch":false,"consensusBeans":null}"#;

        let bean: TaxonomyBean = serde_json::from_str(json).unwrap();

        assert_eq!(bean.reached_rank, LinnaeanRank::Species);
    }

    #[test]
    fn taxonomy_bean_serde_round_trip_uses_camel_case() {
        let original = TaxonomyBean {
            max_allowed_rank: Some(LinnaeanRank::Genus),
            taxonomy: Some("d__2;s__9".to_string()),
            ..bean(LinnaeanRank::Species, "9")
        };

        let value = serde_json::to_value(&original).unwrap();

        assert_eq!(value["reachedRank"], "species");
        assert_eq!(value["maxAllowedRank"], "genus");
        assert_eq!(value["singleMatch"], false);
        assert!(value.get("consensusBeans").is_some());
        assert_eq!(
            serde_json::from_value::<TaxonomyBean>(value).unwrap(),
            original
        );
    }
}
