use self::Taxon::*;
use super::linnaean_ranks::{
    LinnaeanRank::*,
    RankedLinnaeanIdentity::{self, *},
};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    str::{self, FromStr},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomTaxon {
    domain: i16,
    kingdom: Option<i16>,
    phylum: Option<i16>,
    class: Option<i16>,
    order: Option<i16>,
    family: Option<i16>,
    genus: Option<i16>,
    species: i16,
}

impl CustomTaxon {
    pub fn from_file(path: PathBuf) -> Self {
        let extension = path
            .extension()
            .expect("File must have an extension")
            .to_str()
            .expect("Extension must be a valid UTF-8 string");

        if !matches!(extension, "yaml" | "json") {
            panic!("Custom taxon file must be a YAML or JSON file");
        }

        let reader = match std::fs::File::open(path.to_owned()) {
            Ok(file) => file,
            Err(err) => {
                panic!("Could not open custom taxon file: {err}")
            }
        };

        if extension == "yaml" {
            return match serde_yaml::from_reader(&reader) {
                Ok(custom_taxon) => custom_taxon,
                Err(err) => {
                    panic!("Could not parse custom taxon file from YAML: {err}")
                }
            };
        }

        if extension == "json" {
            return match serde_json::from_reader(&reader) {
                Ok(custom_taxon) => custom_taxon,
                Err(err) => {
                    panic!("Could not parse custom taxon file from JSON: {err}")
                }
            };
        }

        panic!("Custom taxon file must be a YAML or JSON file");
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "camelCase")]
pub enum Taxon {
    /// Fungi cutoff values based on default Internal Transcribed Spacer (ITS)
    /// cutoffs.
    ///
    Fungi,

    /// Bacteria cutoff values are based on default 16S rRNA cutoffs.
    ///
    Bacteria,

    /// Eukaryotes cutoff values are based on fungal Internal Transcribed Spacer
    /// (ITS) cutoffs.
    ///
    Eukaryotes,

    /// Custom values must be provided by the user.
    ///
    Custom,
}

impl FromStr for Taxon {
    type Err = ();

    fn from_str(input: &str) -> Result<Taxon, Self::Err> {
        match input {
            "f" | "Fungi" | "fungi" => Ok(Taxon::Fungi),
            "b" | "Bacteria" | "bacteria" => Ok(Taxon::Bacteria),
            "e" | "Eukaryotes" | "eukaryotes" => Ok(Taxon::Eukaryotes),
            "c" | "Custom" | "custom" => Ok(Taxon::Custom),
            _ => Err(()),
        }
    }
}

impl Taxon {
    pub(super) fn get_taxon_cutoff(
        self,
        custom_taxon_values: Option<CustomTaxon>,
    ) -> Vec<RankedLinnaeanIdentity> {
        match self {
            Fungi => Self::get_fungal_cutoffs(),
            Bacteria => Self::get_bacterial_cutoffs(),
            Eukaryotes => Self::get_eukaryote_cutoffs(),
            Custom => match custom_taxon_values {
                Some(custom_taxon_values) => {
                    Self::get_custom_cutoffs(custom_taxon_values)
                }
                None => panic!("Custom taxon values are required"),
            },
        }
    }

    /// Filter custom ranks by identity percentage
    fn get_custom_cutoffs(
        custom_taxon_values: CustomTaxon,
    ) -> Vec<RankedLinnaeanIdentity> {
        vec![
            DefaultRank(Domain, custom_taxon_values.domain as f64),
            DefaultRank(
                Kingdom,
                custom_taxon_values.kingdom.unwrap_or(0) as f64,
            ),
            DefaultRank(Phylum, custom_taxon_values.phylum.unwrap_or(0) as f64),
            DefaultRank(Class, custom_taxon_values.class.unwrap_or(0) as f64),
            DefaultRank(Order, custom_taxon_values.order.unwrap_or(0) as f64),
            DefaultRank(Family, custom_taxon_values.family.unwrap_or(0) as f64),
            DefaultRank(Genus, custom_taxon_values.genus.unwrap_or(0) as f64),
            DefaultRank(Species, custom_taxon_values.species as f64),
        ]
    }

    /// Filter fungi ranks by identity percentage
    ///
    /// TODO: Review the identity percentages and check a reference.
    fn get_fungal_cutoffs() -> Vec<RankedLinnaeanIdentity> {
        vec![
            DefaultRank(Species, 97.0),
            DefaultRank(Genus, 95.0),
            DefaultRank(Family, 90.0),
            DefaultRank(Order, 85.0),
            DefaultRank(Class, 80.0),
            DefaultRank(Phylum, 75.0),
            DefaultRank(Domain, 60.0),
        ]
    }

    /// Filter bacteria ranks by identity percentage
    ///
    /// TODO: Review the identity percentages and check a reference.
    fn get_bacterial_cutoffs() -> Vec<RankedLinnaeanIdentity> {
        vec![
            DefaultRank(Species, 99.0),
            DefaultRank(Genus, 97.0),
            DefaultRank(Family, 92.0),
            DefaultRank(Order, 85.0),
            DefaultRank(Class, 80.0),
            DefaultRank(Phylum, 75.0),
            DefaultRank(Domain, 60.0),
        ]
    }

    /// Filter general eukaryotes ranks by identity percentage
    ///
    /// TODO: Review the identity percentages and check a reference.
    fn get_eukaryote_cutoffs() -> Vec<RankedLinnaeanIdentity> {
        vec![
            DefaultRank(Species, 97.0),
            DefaultRank(Genus, 95.0),
            DefaultRank(Family, 90.0),
            DefaultRank(Order, 85.0),
            DefaultRank(Class, 80.0),
            DefaultRank(Phylum, 75.0),
            DefaultRank(Domain, 60.0),
        ]
    }
}
