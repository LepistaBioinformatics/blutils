use self::LinnaeanRanks::*;

use core::fmt;
use serde::Serialize;
use slugify::slugify;
use std::slice::Iter;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum LinnaeanRanks {
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

impl LinnaeanRanks {
    pub fn ordered_iter(rev: Option<bool>) -> Iter<'static, LinnaeanRanks> {
        let rev = rev.unwrap_or(false);

        if rev {
            static TAXONOMIES: [LinnaeanRanks; 8] = [
                Domain, Kingdom, Phylum, Class, Order, Family, Genus, Species,
            ];

            return TAXONOMIES.iter();
        }

        static TAXONOMIES: [LinnaeanRanks; 9] = [
            Species, Genus, Family, Order, Class, Phylum, Kingdom, Domain,
            Undefined,
        ];

        TAXONOMIES.iter()
    }
}

impl FromStr for LinnaeanRanks {
    type Err = String;

    fn from_str(input: &str) -> Result<LinnaeanRanks, Self::Err> {
        let binding = input.to_lowercase();
        let trimmed_input = binding.trim();

        match trimmed_input {
            "u" | "undefined" => Ok(LinnaeanRanks::Undefined),
            "d" | "domain" => Ok(LinnaeanRanks::Domain),
            "k" | "kingdom" => Ok(LinnaeanRanks::Kingdom),
            "p" | "phylum" => Ok(LinnaeanRanks::Phylum),
            "c" | "class" => Ok(LinnaeanRanks::Class),
            "o" | "order" => Ok(LinnaeanRanks::Order),
            "f" | "family" => Ok(LinnaeanRanks::Family),
            "g" | "genus" => Ok(LinnaeanRanks::Genus),
            "s" | "species" => Ok(LinnaeanRanks::Species),
            other => Ok(LinnaeanRanks::Other(slugify!(other))),
        }
    }
}

impl fmt::Display for LinnaeanRanks {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LinnaeanRanks::Domain => write!(f, "d"),
            LinnaeanRanks::Kingdom => write!(f, "k"),
            LinnaeanRanks::Phylum => write!(f, "p"),
            LinnaeanRanks::Class => write!(f, "c"),
            LinnaeanRanks::Order => write!(f, "o"),
            LinnaeanRanks::Family => write!(f, "f"),
            LinnaeanRanks::Genus => write!(f, "g"),
            LinnaeanRanks::Species => write!(f, "s"),
            LinnaeanRanks::Undefined => write!(f, "u"),
            LinnaeanRanks::Other(other) => write!(f, "{}", other),
        }
    }
}
