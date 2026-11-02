use super::{
    linnaean_ranks::LinnaeanRank,
    taxonomy_bean::{Taxonomy, TaxonomyBean},
};

use mycelium_base::utils::errors::{invalid_arg_err, MappedErrors};
use serde::Serialize;
use tracing::error;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BlastResultRow {
    pub subject_accession: String,
    pub subject_taxid: i64,
    pub perc_identity: f64,
    pub align_length: i64,
    pub mismatches: i64,
    pub gap_openings: i64,
    pub q_start: i64,
    pub q_end: i64,
    pub s_start: i64,
    pub s_end: i64,
    pub e_value: f64,
    pub bit_score: f64,
    pub taxonomy: Taxonomy,
}

impl BlastResultRow {
    ///
    /// Parse taxonomy as a Vec<TaxonomyElement>
    ///
    /// Originally taxonomies has a literal string format like this:
    ///     d__2;p__201174;c__1760;o__85006;f__1268;g__1742989;s__257984
    ///
    /// After execute parsing the literal string should be converted to a
    /// Vec<TaxonomyElement> format.
    ///
    pub fn parse_taxonomy(&mut self) -> Result<Self, MappedErrors> {
        if let Taxonomy::Literal(res) = &self.taxonomy {
            let splitted_taxonomy = res
                //
                // Split by semicolon converting the literal string to a vector
                // of strings containing the rank and the taxid joined with a
                // double underscore. Example:
                //
                //  s__257984
                //
                .split(";")
                .collect::<Vec<_>>();

            let parsed_taxonomy = splitted_taxonomy.to_owned()
                    .into_iter()
                    .filter_map(|tax| {
                    //
                    // Split rank and taxid.
                    //
                    let splitted_tax = tax
                        .split("__")
                        .map(|i| i.to_string())
                        .collect::<Vec<String>>();
                    //
                    // Case after splitted the resulting vector length differs
                    // from two, panic the program.
                    //
                    if splitted_tax.len() != 2 {
                        return None
                    }
                    //
                    // Then, try to parse the resulting vector into a
                    // `TaxonomyElement` struct.
                    //
                    Some(TaxonomyBean {
                        reached_rank: match splitted_tax[0]
                            .to_owned()
                            .parse::<LinnaeanRank>()
                        {
                            Err(err) => {
                                error!(
                                    "Unexpected error on parse taxonomy `{:?}`: {err}",
                                    splitted_tax
                                );

                                return None
                            },
                            Ok(res) => res,
                        },
                        max_allowed_rank: None,
                        identifier: match splitted_tax[1].to_owned().parse::<String>() {
                            Err(err) => {
                                error!(
                                    "Unexpected error on parse taxid `{:?}`: {err}",
                                    splitted_tax
                                );

                                return None;
                            }
                            Ok(res) => res,
                        },
                        perc_identity: self.perc_identity,
                        bit_score: self.bit_score,
                        taxonomy: None,
                        mutated: false,
                        single_match: false,
                        consensus_beans: None,
                    })
                })
                .collect::<Vec<TaxonomyBean>>();

            if parsed_taxonomy.len() != splitted_taxonomy.len() {
                return invalid_arg_err(
                    "Unexpected error on parse taxonomy".to_string(),
                )
                .as_error();
            }

            self.taxonomy = Taxonomy::Parsed(parsed_taxonomy);
        };

        Ok(self.to_owned())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BlastQueryResult {
    pub query: String,
    pub results: Option<Vec<BlastResultRow>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::row;

    fn parsed_beans(row: &BlastResultRow) -> Vec<TaxonomyBean> {
        match &row.taxonomy {
            Taxonomy::Parsed(beans) => beans.clone(),
            Taxonomy::Literal(literal) => {
                panic!("expected parsed taxonomy, got literal {literal}")
            }
        }
    }

    #[test]
    fn parse_taxonomy_full_lineage_returns_seven_beans() {
        let mut hit = row(
            "NR_1.1",
            93973,
            98.5,
            420,
            "d__2;p__1224;c__1236;o__91347;f__543;g__561;s__93973",
        );

        let parsed = hit.parse_taxonomy().unwrap();
        let beans = parsed_beans(&parsed);

        assert_eq!(beans.len(), 7);
        assert_eq!(
            beans
                .iter()
                .map(|b| b.reached_rank.clone())
                .collect::<Vec<_>>(),
            vec![
                LinnaeanRank::Domain,
                LinnaeanRank::Phylum,
                LinnaeanRank::Class,
                LinnaeanRank::Order,
                LinnaeanRank::Family,
                LinnaeanRank::Genus,
                LinnaeanRank::Species,
            ]
        );
        assert_eq!(
            beans
                .iter()
                .map(|b| b.identifier.as_str())
                .collect::<Vec<_>>(),
            vec!["2", "1224", "1236", "91347", "543", "561", "93973"]
        );
    }

    #[test]
    fn parse_taxonomy_bean_copies_scores_and_defaults_other_fields() {
        let mut hit = row("NR_1.1", 2, 98.5, 420, "d__2;g__561");

        let beans = parsed_beans(&hit.parse_taxonomy().unwrap());

        for bean in beans {
            assert_eq!(bean.perc_identity, 98.5);
            assert_eq!(bean.bit_score, 420.0);
            assert_eq!(bean.max_allowed_rank, None);
            assert_eq!(bean.taxonomy, None);
            assert!(!bean.mutated);
            assert!(!bean.single_match);
            assert_eq!(bean.consensus_beans, None);
        }
    }

    #[test]
    fn parse_taxonomy_identifier_kept_raw() {
        let mut hit = row("NR_1.1", 2, 99.0, 100, "g__Bacillus Subtilis");

        let beans = parsed_beans(&hit.parse_taxonomy().unwrap());

        assert_eq!(beans[0].identifier, "Bacillus Subtilis");
    }

    #[test]
    fn parse_taxonomy_non_default_rank_returns_other_slug() {
        let mut hit = row("NR_1.1", 2, 99.0, 100, "species-group__x");

        let beans = parsed_beans(&hit.parse_taxonomy().unwrap());

        assert_eq!(
            beans[0].reached_rank,
            LinnaeanRank::Other("species-group".to_string())
        );
        assert_eq!(beans[0].identifier, "x");
    }

    #[test]
    fn parse_taxonomy_malformed_lineage_returns_err() {
        for lineage in ["", "null", "d__2;", "s__a__b", ";d__2"] {
            let mut hit = row("NR_1.1", 2, 99.0, 100, lineage);

            assert!(
                hit.parse_taxonomy().is_err(),
                "lineage {lineage:?} should be rejected"
            );
        }
    }

    #[test]
    fn parse_taxonomy_err_leaves_self_literal() {
        let mut hit = row("NR_1.1", 2, 99.0, 100, "d__2;");

        assert!(hit.parse_taxonomy().is_err());
        assert!(
            matches!(&hit.taxonomy, Taxonomy::Literal(literal) if literal == "d__2;")
        );
    }

    #[test]
    fn parse_taxonomy_literal_mutates_self_to_parsed() {
        let mut hit = row("NR_1.1", 2, 99.0, 100, "d__2;s__9");

        let returned = hit.parse_taxonomy().unwrap();

        assert_eq!(parsed_beans(&hit), parsed_beans(&returned));
        assert_eq!(parsed_beans(&hit).len(), 2);
    }

    #[test]
    fn parse_taxonomy_already_parsed_returns_unchanged_clone() {
        let mut hit = row("NR_1.1", 2, 99.0, 100, "d__2;s__9");
        let first = parsed_beans(&hit.parse_taxonomy().unwrap());

        let second = hit.parse_taxonomy().unwrap();

        assert_eq!(parsed_beans(&second), first);
        assert_eq!(parsed_beans(&hit), first);
        assert_eq!(second.subject_accession, "NR_1.1");
    }
}
