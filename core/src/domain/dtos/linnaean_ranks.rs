use self::{LinnaeanRank::*, RankedLinnaeanIdentity::*};
use super::blast_builder::Taxon::{self, *};
use super::taxonomy_bean::TaxonomyBean;
use crate::domain::utils::round;

use core::fmt;
use mycelium_base::utils::errors::MappedErrors;
use serde::{Deserialize, Serialize};
use slugify::slugify;
use std::collections::HashMap;
use std::slice::Iter;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LinnaeanRank {
    Undefined,
    Domain,
    Kingdom,
    Phylum,
    Class,
    Order,
    Family,
    Genus,
    Species,

    #[serde(untagged)]
    Other(String),
}

impl LinnaeanRank {
    pub fn ordered_iter(rev: Option<bool>) -> Iter<'static, LinnaeanRank> {
        let rev = rev.unwrap_or(false);

        if rev {
            static TAXONOMIES: [LinnaeanRank; 8] = [
                Domain, Kingdom, Phylum, Class, Order, Family, Genus, Species,
            ];

            return TAXONOMIES.iter();
        }

        static TAXONOMIES: [LinnaeanRank; 9] = [
            Species, Genus, Family, Order, Class, Phylum, Kingdom, Domain,
            Undefined,
        ];

        TAXONOMIES.iter()
    }
}

impl FromStr for LinnaeanRank {
    type Err = String;

    fn from_str(input: &str) -> Result<LinnaeanRank, Self::Err> {
        let binding = input.to_lowercase();
        let trimmed_input = binding.trim();

        match trimmed_input {
            "u" | "undefined" => Ok(Undefined),
            "d" | "domain" => Ok(Domain),
            "k" | "kingdom" => Ok(Kingdom),
            "p" | "phylum" => Ok(Phylum),
            "c" | "class" => Ok(Class),
            "o" | "order" => Ok(Order),
            "f" | "family" => Ok(Family),
            "g" | "genus" => Ok(Genus),
            "s" | "species" => Ok(Species),
            other => Ok(Other(slugify!(other))),
        }
    }
}

impl fmt::Display for LinnaeanRank {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Domain => write!(f, "d"),
            Kingdom => write!(f, "k"),
            Phylum => write!(f, "p"),
            Class => write!(f, "c"),
            Order => write!(f, "o"),
            Family => write!(f, "f"),
            Genus => write!(f, "g"),
            Species => write!(f, "s"),
            Undefined => write!(f, "u"),
            Other(other) => write!(f, "{}", other),
        }
    }
}

impl LinnaeanRank {
    pub(crate) fn as_full_rank_string(&self) -> String {
        match self {
            Domain => "domain",
            Kingdom => "kingdom",
            Phylum => "phylum",
            Class => "class",
            Order => "order",
            Family => "family",
            Genus => "genus",
            Species => "species",
            Undefined => "undefined",
            Other(other) => other,
        }
        .to_string()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum RankedLinnaeanIdentity {
    DefaultRank(LinnaeanRank, f64),
    NonDefaultRank(String, f64),
}

impl fmt::Display for RankedLinnaeanIdentity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DefaultRank(rank, identity) => {
                write!(f, "{}:{}", rank, identity)
            }
            NonDefaultRank(rank, identity) => {
                write!(f, "{}:{}", rank, identity)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InterpolatedIdentity {
    interpolation: Vec<RankedLinnaeanIdentity>,
}

impl fmt::Display for InterpolatedIdentity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let interpolation = self
            .interpolation
            .iter()
            .map(|rank| rank.to_string())
            .collect::<Vec<String>>()
            .join(",");

        write!(f, "{}", interpolation)
    }
}

impl InterpolatedIdentity {
    /// Create a new interpolated identity
    ///
    /// This function should be used to create a new interpolated identity. The
    /// interpolation is based on the taxon and the taxonomy vector.
    ///
    pub(crate) fn new(
        taxon: Taxon,
        taxonomy: Vec<LinnaeanRank>,
    ) -> Result<Self, MappedErrors> {
        let interpolation = Self::interpolate_identities(taxon, taxonomy)?;
        Ok(Self { interpolation })
    }

    /// Get the interpolation
    pub(crate) fn interpolation(&self) -> &Vec<RankedLinnaeanIdentity> {
        &self.interpolation
    }

    /// Get the rank adjusted by identity
    ///
    /// This function should be used to get the rank adjusted by identity. The
    /// identity is used after the interpolation (executed by the `new` method).
    ///
    pub(crate) fn get_rank_adjusted_by_identity(
        &self,
        identity: f64,
    ) -> Option<RankedLinnaeanIdentity> {
        let mut result =
            self.interpolation
                .to_owned()
                .into_iter()
                .skip_while(|rank| {
                    let rank_identity = match rank {
                        DefaultRank(_, rank_identity) => *rank_identity,
                        NonDefaultRank(_, rank_identity) => *rank_identity,
                    };

                    identity > rank_identity
                });

        result.find_map(|rank| Some(rank))
    }

    pub(crate) fn get_adjusted_taxonomy_by_identity(
        &self,
        identity: f64,
        taxonomy: Vec<TaxonomyBean>,
    ) -> Vec<TaxonomyBean> {
        self.interpolation()
            .into_iter()
            .zip(taxonomy.to_owned().into_iter())
            .filter(|(interpolated, _)| {
                let item_identity = match interpolated {
                    DefaultRank(_, perc_identity) => perc_identity,
                    NonDefaultRank(_, perc_identity) => perc_identity,
                };

                &identity >= item_identity
            })
            .map(|(_, taxonomy)| taxonomy)
            .collect()
    }

    ///
    /// Interpolate identity percentages
    ///
    /// This function should be used to interpolate the identity percentage to
    /// provide identity cutoffs to non-linnaean ranks, such as strains,
    /// serovars, etc. Such ranks are stored as Other in the `LinnaeanRank` enum.
    fn interpolate_identities(
        taxon: Taxon,
        taxonomy: Vec<LinnaeanRank>,
    ) -> Result<Vec<RankedLinnaeanIdentity>, MappedErrors> {
        //
        // Collect the backbone for the taxon
        //
        // The backbone is a vector of `RankedLinnaeanIdentity` that contains
        // the identity cutoffs for the default Linnaeus taxon ranks.
        //
        let backbone = Self::get_taxon_cutoff(taxon);
        //
        // Map taxonomies to the backbone
        //
        // It is important to note that the taxonomy vector should be ordered
        // from the most specific rank to the most general rank. Here ranks non
        // mapped to the backbone vector should be interpolated.
        //
        let backbone_mapped_taxonomies = taxonomy
            .iter()
            .map(|rank| {
                //
                // Build default rank
                //
                let binding = NonDefaultRank(rank.to_string(), 0.0);
                //
                // Find the rank in the taxonomy backbone or use the binding
                //
                let ranked_taxon = backbone
                    .iter()
                    .find(|&level| match level {
                        DefaultRank(level_rank, _) => level_rank == rank,
                        _ => false,
                    })
                    .unwrap_or(&binding);
                //
                // Return the rank
                //
                ranked_taxon.to_owned()
            })
            .collect::<Vec<_>>();
        //
        // If the taxonomy has only default ranks, return the backbone
        //
        if backbone_mapped_taxonomies.iter().all(|rank| match rank {
            DefaultRank(_, _) => true,
            _ => false,
        }) {
            return Ok(backbone_mapped_taxonomies);
        }
        //
        // Map the non-default ranks windows as a vector of tuples containing
        // the window which the non-default rank is located and the rank itself.
        //
        let non_default_ranks = backbone_mapped_taxonomies
            .iter()
            .enumerate()
            .filter(|(_, rank)| match rank {
                NonDefaultRank(_, _) => true,
                _ => false,
            })
            .collect::<Vec<_>>();
        //
        // Map the previous and next default ranks for each non-default rank.
        // Non-default ranks should occur in sequences of more than one
        // non-default rank. Thus, the search should be extended to the next
        // default rank.
        //
        let non_default_ranks_windows = non_default_ranks
            .into_iter()
            .map(|(non_default_index, _)| {
                let previous = backbone_mapped_taxonomies
                    .iter()
                    .take(non_default_index)
                    .rev()
                    .find(|&level| match level {
                        NonDefaultRank(_, _) => false,
                        _ => true,
                    })
                    .unwrap_or(&backbone_mapped_taxonomies[0]);

                let previous_index = backbone_mapped_taxonomies
                    .iter()
                    .position(|level| level == previous)
                    .unwrap_or(0);

                let next = backbone_mapped_taxonomies
                    .iter()
                    .skip(non_default_index)
                    .find(|&level| match level {
                        NonDefaultRank(_, _) => false,
                        _ => true,
                    })
                    .unwrap_or(
                        &backbone_mapped_taxonomies
                            [backbone_mapped_taxonomies.len() - 1],
                    );

                let next_index = backbone_mapped_taxonomies
                    .iter()
                    .position(|level| level == next)
                    .unwrap_or(backbone_mapped_taxonomies.len() - 1);

                let window = backbone_mapped_taxonomies
                    .iter()
                    .skip_while(|&level| level != previous)
                    .take(next_index + 1)
                    .cloned()
                    .collect::<Vec<_>>();

                (non_default_index, (previous_index, next_index), window)
            })
            .collect::<Vec<_>>();

        let mut updated_identities = HashMap::<i32, f64>::new();
        for (non_default_index, (previous_index, _), window) in
            non_default_ranks_windows
        {
            let target_index = non_default_index - previous_index;

            let first_window_identity = match window[0].to_owned() {
                DefaultRank(_, identity) => identity,
                NonDefaultRank(_, _) => match backbone[0] {
                    DefaultRank(_, default_identity) => default_identity,
                    _ => panic!("Unexpected error. Could not determine default identity"),
                },
            };

            let last_window_identity = match window[window.len() - 1].to_owned()
            {
                DefaultRank(_, identity) => identity,
                NonDefaultRank(_, _) => 100.0,
            };

            /* if last_window_identity < first_window_identity {
                panic!("Unexpected error. Last window identity is less than first window identity");
            } */

            let window_weight = last_window_identity - first_window_identity;
            let window_size = (window.len() - 1) as f64;

            let target_identity = round(
                first_window_identity +
                    (target_index as f64 * (window_weight / window_size)),
                3,
            );

            updated_identities
                .insert(non_default_index as i32, target_identity);
        }

        Ok(backbone_mapped_taxonomies
            .into_iter()
            .enumerate()
            .map(|(index, item)| match item {
                NonDefaultRank(rank, _) => {
                    let identity = updated_identities
                        .get(&(index as i32))
                        .unwrap_or(&100.0)
                        .to_owned();

                    NonDefaultRank(rank, identity)
                }
                _ => item,
            })
            .collect::<Vec<_>>())
    }

    fn get_taxon_cutoff(taxon: Taxon) -> Vec<RankedLinnaeanIdentity> {
        match taxon {
            Fungi => Self::get_fungal_cutoffs(),
            Bacteria => Self::get_bacterial_cutoffs(),
            Eukaryotes => Self::get_eukaryote_cutoffs(),
        }
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
