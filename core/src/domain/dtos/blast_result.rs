use self::ValidTaxonomicRanksEnum::*;

use core::fmt;
use mycelium_base::utils::errors::{invalid_arg_err, MappedErrors};
use serde::Serialize;
use slugify::slugify;
use std::slice::Iter;
use std::str::FromStr;
use tracing::error;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ValidTaxonomicRanksEnum {
    Undefined,
    Domain,
    Kingdom,
    Phylum,
    Class,
    Order,
    Family,
    Genus,
    Species,
    Other(String),
}

impl ValidTaxonomicRanksEnum {
    pub fn ordered_iter(
        rev: Option<bool>,
    ) -> Iter<'static, ValidTaxonomicRanksEnum> {
        let rev = rev.unwrap_or(false);

        if rev {
            static TAXONOMIES: [ValidTaxonomicRanksEnum; 8] = [
                Domain, Kingdom, Phylum, Class, Order, Family, Genus, Species,
            ];

            return TAXONOMIES.iter();
        }

        static TAXONOMIES: [ValidTaxonomicRanksEnum; 9] = [
            Species, Genus, Family, Order, Class, Phylum, Kingdom, Domain,
            Undefined,
        ];

        TAXONOMIES.iter()
    }
}

impl FromStr for ValidTaxonomicRanksEnum {
    type Err = String;

    fn from_str(input: &str) -> Result<ValidTaxonomicRanksEnum, Self::Err> {
        let binding = input.to_lowercase();
        let trimmed_input = binding.trim();

        match trimmed_input {
            "u" | "undefined" => Ok(ValidTaxonomicRanksEnum::Undefined),
            "d" | "domain" => Ok(ValidTaxonomicRanksEnum::Domain),
            "k" | "kingdom" => Ok(ValidTaxonomicRanksEnum::Kingdom),
            "p" | "phylum" => Ok(ValidTaxonomicRanksEnum::Phylum),
            "c" | "class" => Ok(ValidTaxonomicRanksEnum::Class),
            "o" | "order" => Ok(ValidTaxonomicRanksEnum::Order),
            "f" | "family" => Ok(ValidTaxonomicRanksEnum::Family),
            "g" | "genus" => Ok(ValidTaxonomicRanksEnum::Genus),
            "s" | "species" => Ok(ValidTaxonomicRanksEnum::Species),
            other => Ok(ValidTaxonomicRanksEnum::Other(slugify!(other))),
        }
    }
}

impl fmt::Display for ValidTaxonomicRanksEnum {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValidTaxonomicRanksEnum::Domain => write!(f, "d"),
            ValidTaxonomicRanksEnum::Kingdom => write!(f, "k"),
            ValidTaxonomicRanksEnum::Phylum => write!(f, "p"),
            ValidTaxonomicRanksEnum::Class => write!(f, "c"),
            ValidTaxonomicRanksEnum::Order => write!(f, "o"),
            ValidTaxonomicRanksEnum::Family => write!(f, "f"),
            ValidTaxonomicRanksEnum::Genus => write!(f, "g"),
            ValidTaxonomicRanksEnum::Species => write!(f, "s"),
            ValidTaxonomicRanksEnum::Undefined => write!(f, "u"),
            ValidTaxonomicRanksEnum::Other(other) => write!(f, "{}", other),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaxonomyElement {
    pub rank: ValidTaxonomicRanksEnum,
    pub taxid: i64,
    pub perc_identity: f64,
    pub taxonomy: Option<String>,
    pub mutated: bool,
}

impl TaxonomyElement {
    pub fn taxonomy_to_string(&self) -> String {
        format!("{}__{}", self.rank.to_string(), self.taxid.to_string())
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum TaxonomyFieldEnum {
    Literal(String),
    Parsed(Vec<TaxonomyElement>),
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BlastResultRow {
    pub subject: String,
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
    pub taxonomy: TaxonomyFieldEnum,
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
        if let TaxonomyFieldEnum::Literal(res) = &self.taxonomy {
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
                    Some(TaxonomyElement {
                        rank: match splitted_tax[0]
                            .to_owned()
                            .parse::<ValidTaxonomicRanksEnum>()
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
                        taxid: match splitted_tax[1].to_owned().parse::<i64>() {
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
                        taxonomy: None,
                        mutated: false,
                    })
                })
                .collect::<Vec<TaxonomyElement>>();

            if parsed_taxonomy.len() != splitted_taxonomy.len() {
                return invalid_arg_err(
                    "Unexpected error on parse taxonomy".to_string(),
                )
                .as_error();
            }

            self.taxonomy = TaxonomyFieldEnum::Parsed(parsed_taxonomy);
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

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BlastQueryConsensusResult {
    pub query: String,
    pub taxon: Option<TaxonomyElement>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlastQueryNoConsensusResult {
    pub query: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ConsensusResult {
    /// No consensus option
    ///
    /// This option should be used when the consensus checking process not found
    /// an appropriate taxonomy.
    NoConsensusFound(BlastQueryNoConsensusResult),

    /// Consensus option
    ///
    /// This option should be used when the consensus checking process found an
    /// appropriate taxonomy.
    ConsensusFound(BlastQueryConsensusResult),
}
