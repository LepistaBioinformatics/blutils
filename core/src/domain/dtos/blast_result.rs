use self::ValidTaxonomicRanksEnum::*;

use serde::Serialize;
use std::slice::Iter;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ValidTaxonomicRanksEnum {
    Undefined,
    Domain,
    Phylum,
    Class,
    Order,
    Family,
    Genus,
    Species,
}

impl ValidTaxonomicRanksEnum {
    pub fn ordered_iter(
        rev: Option<bool>,
    ) -> Iter<'static, ValidTaxonomicRanksEnum> {
        let rev = rev.unwrap_or(false);

        if rev {
            static TAXONOMIES: [ValidTaxonomicRanksEnum; 7] =
                [Domain, Phylum, Class, Order, Family, Genus, Species];

            return TAXONOMIES.iter();
        }

        static TAXONOMIES: [ValidTaxonomicRanksEnum; 8] = [
            Species, Genus, Family, Order, Class, Phylum, Domain, Undefined,
        ];

        TAXONOMIES.iter()
    }
}

impl FromStr for ValidTaxonomicRanksEnum {
    type Err = ();

    fn from_str(input: &str) -> Result<ValidTaxonomicRanksEnum, Self::Err> {
        match input {
            "u" | "Undefined" | "undefined" => {
                Ok(ValidTaxonomicRanksEnum::Undefined)
            }
            "d" | "Domain" | "domain" => Ok(ValidTaxonomicRanksEnum::Domain),
            "p" | "Phylum" | "phylum" => Ok(ValidTaxonomicRanksEnum::Phylum),
            "c" | "Class" | "class" => Ok(ValidTaxonomicRanksEnum::Class),
            "o" | "Order" | "order" => Ok(ValidTaxonomicRanksEnum::Order),
            "f" | "Family" | "family" => Ok(ValidTaxonomicRanksEnum::Family),
            "g" | "Genus" | "genus" => Ok(ValidTaxonomicRanksEnum::Genus),
            "s" | "Species" | "species" => Ok(ValidTaxonomicRanksEnum::Species),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaxonomyElement {
    pub rank: ValidTaxonomicRanksEnum,
    pub taxid: i64,
    pub perc_identity: f64,
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
    pub fn parse_taxonomy(&mut self) -> Self {
        if let TaxonomyFieldEnum::Literal(res) = &self.taxonomy {
            let parsed_taxonomy = res
                //
                // Split by semicolon converting the literal string to a vector
                // of strings containing the rank and the taxid joined with a
                // double underscore. Example:
                //
                //  s__257984
                //
                .split(";")
                .map(|tax| {
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
                        panic!("Invalid taxonomy format.")
                    }
                    //
                    // Then, try to parse the resulting vector into a
                    // `TaxonomyElement` struct.
                    //
                    TaxonomyElement {
                        rank: match splitted_tax[0]
                            .to_owned()
                            .parse::<ValidTaxonomicRanksEnum>()
                        {
                            Err(_) => {
                                panic!("Unexpected error on parse taxonomy.")
                            }
                            Ok(res) => res,
                        },
                        taxid: match splitted_tax[1].to_owned().parse::<i64>() {
                            Err(err) => {
                                panic!("Unexpected error on parse taxid: {err}")
                            }
                            Ok(res) => res,
                        },
                        perc_identity: self.perc_identity,
                    }
                })
                .collect();

            self.taxonomy = TaxonomyFieldEnum::Parsed(parsed_taxonomy);
        };

        return self.to_owned();
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
