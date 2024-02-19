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
    pub bit_score: i64,
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
                        bit_score: self.bit_score as f64,
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
