use serde::Serialize;

use self::ValidTaxonomicRanksEnum::*;
use std::slice::Iter;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum ValidTaxonomicRanksEnum {
    Domain,
    Phylum,
    Class,
    Order,
    Family,
    Genus,
    Species,
}

impl ValidTaxonomicRanksEnum {
    pub fn ordered_iter() -> Iter<'static, ValidTaxonomicRanksEnum> {
        static TAXONOMIES: [ValidTaxonomicRanksEnum; 7] =
            [Species, Genus, Family, Order, Class, Phylum, Domain];
        TAXONOMIES.iter()
    }
}

impl FromStr for ValidTaxonomicRanksEnum {
    type Err = ();

    fn from_str(input: &str) -> Result<ValidTaxonomicRanksEnum, Self::Err> {
        match input {
            "d" | "Domain" | "domain" => Ok(ValidTaxonomicRanksEnum::Domain),
            "p" | "Phylum" | "phylum" => Ok(ValidTaxonomicRanksEnum::Domain),
            "c" | "Class" | "class" => Ok(ValidTaxonomicRanksEnum::Class),
            "o" | "Order" | "order" => Ok(ValidTaxonomicRanksEnum::Order),
            "f" | "Family" | "family" => Ok(ValidTaxonomicRanksEnum::Family),
            "g" | "Genus" | "genus" => Ok(ValidTaxonomicRanksEnum::Genus),
            "s" | "Species" | "species" => Ok(ValidTaxonomicRanksEnum::Species),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct TaxonomyElement {
    pub rank: ValidTaxonomicRanksEnum,
    pub taxid: i64,
}

#[derive(Clone, Debug)]
pub enum TaxonomyFieldEnum {
    Literal(String),
    Parser(Vec<TaxonomyElement>),
}

#[derive(Clone, Debug)]
pub struct BlastResultRow {
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
                    }
                })
                .collect();

            self.taxonomy = TaxonomyFieldEnum::Parser(parsed_taxonomy);
        };

        return self.to_owned();
    }
}

#[derive(Debug)]
pub struct BlastQueryResult {
    pub query: String,
    pub results: Option<Vec<BlastResultRow>>,
}

#[derive(Debug, Serialize)]
pub struct BlastQueryPublicResult {
    pub query: String,
    pub taxon: TaxonomyElement,
}

#[derive(Debug)]
pub enum ConsensusResult {
    NoConsensusFound(String),
    Success(BlastQueryPublicResult),
}
