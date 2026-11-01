use self::{LinnaeanRank::*, RankedLinnaeanIdentity::*};
use super::taxon::{CustomTaxon, Taxon};
use super::taxonomy_bean::TaxonomyBean;
use crate::domain::utils::round;

use core::fmt;
use mycelium_base::utils::errors::MappedErrors;
use serde::{de, Deserialize, Deserializer, Serialize};
use slugify::slugify;
use std::collections::HashMap;
use std::slice::Iter;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Serialize)]
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

/// BUG-R18: deserialization goes through `FromStr`, so short rank codes
/// (`"s"`, `"g"`, ...) yield the same variant as parsing them.
impl<'de> Deserialize<'de> for LinnaeanRank {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        LinnaeanRank::from_str(&value).map_err(de::Error::custom)
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
        custom_taxon_values: Option<CustomTaxon>,
    ) -> Result<Self, MappedErrors> {
        let interpolation =
            Self::interpolate_identities(taxon, taxonomy, custom_taxon_values)?;
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
        // BUG-R01: the deepest rank whose cutoff is satisfied (cutoff <=
        // identity) is the maximum allowed rank (FR-IDENT-03).
        self.interpolation
            .iter()
            .rev()
            .find(|rank| {
                let rank_identity = match rank {
                    DefaultRank(_, rank_identity) => *rank_identity,
                    NonDefaultRank(_, rank_identity) => *rank_identity,
                };

                rank_identity <= identity
            })
            .cloned()
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
        custom_taxon_values: Option<CustomTaxon>,
    ) -> Result<Vec<RankedLinnaeanIdentity>, MappedErrors> {
        //
        // Collect the backbone for the taxon
        //
        // The backbone is a vector of `RankedLinnaeanIdentity` that contains
        // the identity cutoffs for the default Linnaeus taxon ranks.
        //
        let backbone = taxon.get_taxon_cutoff(custom_taxon_values);
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
                let is_default = |level: &RankedLinnaeanIdentity| {
                    matches!(level, DefaultRank(_, _))
                };

                let previous_index = (0..non_default_index)
                    .rev()
                    .find(|&index| {
                        is_default(&backbone_mapped_taxonomies[index])
                    })
                    .unwrap_or(0);

                let next_index = (non_default_index
                    ..backbone_mapped_taxonomies.len())
                    .find(|&index| {
                        is_default(&backbone_mapped_taxonomies[index])
                    })
                    .unwrap_or(backbone_mapped_taxonomies.len() - 1);
                //
                // BUG-R05: the window spans exactly from the previous to the
                // next default rank.
                //
                let window = backbone_mapped_taxonomies
                    [previous_index..=next_index]
                    .to_vec();

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

            let window_weight = last_window_identity - first_window_identity;
            let window_size = (window.len() - 1) as f64;

            let target_identity = round(
                first_window_identity
                    + (target_index as f64 * (window_weight / window_size)),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    const TOL: f64 = 1e-9;

    const BACTERIA_RANKS: [&str; 7] = ["d", "p", "c", "o", "f", "g", "s"];

    fn identity_of(rank: &RankedLinnaeanIdentity) -> f64 {
        match rank {
            DefaultRank(_, identity) => *identity,
            NonDefaultRank(_, identity) => *identity,
        }
    }

    fn identities(interpolated: &InterpolatedIdentity) -> Vec<f64> {
        interpolated
            .interpolation()
            .iter()
            .map(identity_of)
            .collect()
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < TOL,
            "expected {expected}, got {actual}"
        );
    }

    fn assert_all_close(actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len(), "{actual:?} vs {expected:?}");
        for (a, e) in actual.iter().zip(expected) {
            assert_close(*a, *e);
        }
    }

    fn custom(json: &str) -> CustomTaxon {
        serde_json::from_str(json).unwrap()
    }

    fn bean(rank: LinnaeanRank, identifier: &str) -> TaxonomyBean {
        TaxonomyBean {
            reached_rank: rank,
            max_allowed_rank: None,
            identifier: identifier.to_string(),
            perc_identity: 0.0,
            bit_score: 0.0,
            taxonomy: None,
            mutated: false,
            single_match: false,
            consensus_beans: None,
        }
    }

    fn bacteria_lineage() -> Vec<TaxonomyBean> {
        ranks(&BACTERIA_RANKS)
            .into_iter()
            .enumerate()
            .map(|(i, rank)| bean(rank, &format!("t{i}")))
            .collect()
    }

    fn ranks_of(beans: &[TaxonomyBean]) -> Vec<LinnaeanRank> {
        beans.iter().map(|b| b.reached_rank.clone()).collect()
    }

    // ---------------------------------------------------------------------
    // LinnaeanRank: FromStr / Display / as_full_rank_string
    // ---------------------------------------------------------------------

    #[test]
    fn from_str_short_and_long_names_map_to_default_ranks() {
        let cases = [
            ("u", Undefined),
            ("undefined", Undefined),
            ("d", Domain),
            ("domain", Domain),
            ("k", Kingdom),
            ("kingdom", Kingdom),
            ("p", Phylum),
            ("phylum", Phylum),
            ("c", Class),
            ("class", Class),
            ("o", Order),
            ("order", Order),
            ("f", Family),
            ("family", Family),
            ("g", Genus),
            ("genus", Genus),
            ("s", Species),
            ("species", Species),
        ];

        for (input, expected) in cases {
            assert_eq!(LinnaeanRank::from_str(input), Ok(expected), "{input}");
        }
    }

    #[test]
    fn from_str_mixed_case_and_whitespace_is_normalized() {
        assert_eq!(LinnaeanRank::from_str(" Species "), Ok(Species));
        assert_eq!(LinnaeanRank::from_str("GENUS"), Ok(Genus));
        assert_eq!(LinnaeanRank::from_str("\tD\n"), Ok(Domain));
    }

    #[test]
    fn from_str_unknown_rank_is_slugified_other() {
        assert_eq!(
            LinnaeanRank::from_str("Species Group"),
            Ok(Other("species-group".to_string()))
        );
        assert_eq!(
            LinnaeanRank::from_str("no rank"),
            Ok(Other("no-rank".to_string()))
        );
        assert_eq!(
            LinnaeanRank::from_str("strain"),
            Ok(Other("strain".to_string()))
        );
    }

    #[test]
    fn from_str_any_input_never_errors() {
        for input in ["", "   ", "__", "çã", "123", "a;b"] {
            assert!(LinnaeanRank::from_str(input).is_ok(), "{input:?}");
        }
    }

    #[test]
    fn display_default_ranks_use_short_names() {
        assert_eq!(Species.to_string(), "s");
        assert_eq!(Domain.to_string(), "d");
        assert_eq!(Undefined.to_string(), "u");
        assert_eq!(Other("strain".to_string()).to_string(), "strain");
    }

    #[test]
    fn as_full_rank_string_default_ranks_use_long_names() {
        assert_eq!(Species.as_full_rank_string(), "species");
        assert_eq!(Kingdom.as_full_rank_string(), "kingdom");
        assert_eq!(Undefined.as_full_rank_string(), "undefined");
        assert_eq!(Other("strain".to_string()).as_full_rank_string(), "strain");
    }

    #[test]
    fn from_str_of_display_all_variants_round_trips() {
        let variants = [
            Undefined,
            Domain,
            Kingdom,
            Phylum,
            Class,
            Order,
            Family,
            Genus,
            Species,
            Other("strain".to_string()),
            Other("species-group".to_string()),
        ];

        for variant in variants {
            assert_eq!(
                LinnaeanRank::from_str(&variant.to_string()),
                Ok(variant.clone())
            );
            assert_eq!(
                LinnaeanRank::from_str(&variant.as_full_rank_string()),
                Ok(variant)
            );
        }
    }

    #[test]
    fn ordered_iter_default_and_reversed_orders() {
        let forward: Vec<_> =
            LinnaeanRank::ordered_iter(None).cloned().collect();
        assert_eq!(
            forward,
            vec![
                Species, Genus, Family, Order, Class, Phylum, Kingdom, Domain,
                Undefined
            ]
        );

        let reversed: Vec<_> =
            LinnaeanRank::ordered_iter(Some(true)).cloned().collect();
        assert_eq!(
            reversed,
            vec![Domain, Kingdom, Phylum, Class, Order, Family, Genus, Species]
        );
    }

    // ---------------------------------------------------------------------
    // LinnaeanRank: serde
    // ---------------------------------------------------------------------

    #[test]
    fn serde_default_rank_serializes_as_camel_case_full_name() {
        assert_eq!(serde_json::to_string(&Species).unwrap(), "\"species\"");
        assert_eq!(
            serde_json::from_str::<LinnaeanRank>("\"species\"").unwrap(),
            Species
        );
    }

    #[test]
    fn serde_other_rank_is_untagged_string() {
        let strain = Other("strain".to_string());
        assert_eq!(serde_json::to_string(&strain).unwrap(), "\"strain\"");
        assert_eq!(
            serde_json::from_str::<LinnaeanRank>("\"strain\"").unwrap(),
            strain
        );
    }

    // BUG-R18: deserializing short rank names matches FromStr.
    #[test]
    fn serde_short_name_deserializes_as_default_rank() {
        assert_eq!(
            serde_json::from_str::<LinnaeanRank>("\"s\"").unwrap(),
            Species
        );
    }

    // BUG-R18: deserialization goes through FromStr, which is
    // case-insensitive, so a capitalized name is no longer `Other`.
    #[test]
    fn serde_capitalized_name_deserializes_as_default_rank() {
        assert_eq!(
            serde_json::from_str::<LinnaeanRank>("\"Species\"").unwrap(),
            Species
        );
    }

    // ---------------------------------------------------------------------
    // InterpolatedIdentity::new (interpolate_identities)
    // ---------------------------------------------------------------------

    #[test]
    fn interpolate_bacteria_default_ranks_returns_backbone() {
        let interpolated = interp(Taxon::Bacteria, &BACTERIA_RANKS);

        assert_all_close(
            &identities(&interpolated),
            &[60.0, 75.0, 80.0, 85.0, 92.0, 97.0, 99.0],
        );
        assert!(interpolated
            .interpolation()
            .iter()
            .all(|r| matches!(r, DefaultRank(_, _))));
    }

    #[test]
    fn interpolate_fungi_and_eukaryotes_default_ranks_returns_backbone() {
        let expected = [60.0, 75.0, 80.0, 85.0, 90.0, 95.0, 97.0];

        assert_all_close(
            &identities(&interp(Taxon::Fungi, &BACTERIA_RANKS)),
            &expected,
        );
        assert_all_close(
            &identities(&interp(Taxon::Eukaryotes, &BACTERIA_RANKS)),
            &expected,
        );
    }

    #[test]
    fn interpolate_default_ranks_keeps_taxonomy_order() {
        let interpolated = interp(Taxon::Bacteria, &["s", "g", "d"]);

        assert_eq!(
            interpolated.interpolation(),
            &vec![
                DefaultRank(Species, 99.0),
                DefaultRank(Genus, 97.0),
                DefaultRank(Domain, 60.0),
            ]
        );
    }

    #[test]
    fn interpolate_empty_taxonomy_returns_empty() {
        let interpolated =
            InterpolatedIdentity::new(Taxon::Bacteria, vec![], None).unwrap();

        assert!(interpolated.interpolation().is_empty());
    }

    #[test]
    fn interpolate_intermediate_non_default_rank_is_linear_between_neighbors() {
        let interpolated = interp(
            Taxon::Bacteria,
            &["d", "p", "c", "o", "f", "g", "species-group", "s"],
        );

        assert_eq!(
            interpolated.interpolation()[6],
            NonDefaultRank("species-group".to_string(), 98.0)
        );
        assert_all_close(
            &identities(&interpolated),
            &[60.0, 75.0, 80.0, 85.0, 92.0, 97.0, 98.0, 99.0],
        );
    }

    // BUG-R05: the interpolation window stops at the next default rank.
    #[test]
    fn interpolate_window_with_offset_is_bounded_by_next_default_rank() {
        let interpolated = interp(
            Taxon::Bacteria,
            &["d", "p", "c", "subclass", "o", "f", "g", "s"],
        );

        assert_close(identity_of(&interpolated.interpolation()[3]), 82.5);
    }

    #[test]
    fn interpolate_trailing_non_default_rank_gets_100() {
        let interpolated = interp(
            Taxon::Bacteria,
            &["d", "p", "c", "o", "f", "g", "s", "strain"],
        );

        assert_eq!(
            interpolated.interpolation()[7],
            NonDefaultRank("strain".to_string(), 100.0)
        );
    }

    #[test]
    fn interpolate_two_trailing_non_default_ranks_ramp_up_to_100() {
        let interpolated = interp(
            Taxon::Bacteria,
            &["d", "p", "c", "o", "f", "g", "s", "subspecies", "strain"],
        );

        assert_all_close(&identities(&interpolated)[7..], &[99.5, 100.0]);
    }

    #[test]
    fn interpolate_leading_non_default_rank_built_in_uses_first_backbone_entry()
    {
        // Built-in backbones start at Species, so a leading non-default rank
        // inherits the species cutoff.
        let interpolated = interp(
            Taxon::Bacteria,
            &["clade", "d", "p", "c", "o", "f", "g", "s"],
        );

        assert_eq!(
            interpolated.interpolation()[0],
            NonDefaultRank("clade".to_string(), 99.0)
        );
    }

    #[test]
    fn interpolate_leading_non_default_rank_custom_uses_domain_cutoff() {
        // Custom backbones start at Domain, unlike the built-in ones.
        let interpolated = InterpolatedIdentity::new(
            Taxon::Custom,
            ranks(&["clade", "d", "p", "c", "o", "f", "g", "s"]),
            Some(custom(
                r#"{"domain":61,"phylum":75,"class":80,"order":85,
                    "family":92,"genus":97,"species":99}"#,
            )),
        )
        .unwrap();

        assert_eq!(
            interpolated.interpolation()[0],
            NonDefaultRank("clade".to_string(), 61.0)
        );
    }

    #[test]
    fn interpolate_single_non_default_rank_yields_nan() {
        let interpolated = interp(Taxon::Bacteria, &["strain"]);

        assert_eq!(interpolated.interpolation().len(), 1);
        assert!(identity_of(&interpolated.interpolation()[0]).is_nan());
    }

    #[test]
    fn interpolate_duplicated_non_default_names_get_independent_windows() {
        let interpolated =
            interp(Taxon::Bacteria, &["d", "clade", "p", "clade", "c"]);

        assert_all_close(
            &identities(&interpolated),
            &[60.0, 67.5, 75.0, 77.5, 80.0],
        );
    }

    // BUG-R17: Kingdom is a default rank of the built-in backbones.
    #[test]
    fn interpolate_kingdom_built_in_is_default_rank() {
        let interpolated = interp(Taxon::Bacteria, &["d", "k", "p"]);

        assert!(matches!(
            interpolated.interpolation()[1],
            DefaultRank(Kingdom, _)
        ));
    }

    #[test]
    fn interpolate_custom_missing_ranks_default_to_zero() {
        let interpolated = InterpolatedIdentity::new(
            Taxon::Custom,
            ranks(&["d", "k", "g", "s"]),
            Some(custom(r#"{"domain":60,"species":99}"#)),
        )
        .unwrap();

        assert_eq!(
            interpolated.interpolation(),
            &vec![
                DefaultRank(Domain, 60.0),
                DefaultRank(Kingdom, 0.0),
                DefaultRank(Genus, 0.0),
                DefaultRank(Species, 99.0),
            ]
        );
    }

    #[test]
    fn display_interpolated_identity_joins_rank_and_cutoff() {
        assert_eq!(
            interp(Taxon::Bacteria, &BACTERIA_RANKS).to_string(),
            "d:60,p:75,c:80,o:85,f:92,g:97,s:99"
        );
        assert_eq!(
            interp(Taxon::Bacteria, &["g", "species-group", "s"]).to_string(),
            "g:97,species-group:98,s:99"
        );
    }

    // ---------------------------------------------------------------------
    // get_rank_adjusted_by_identity
    // ---------------------------------------------------------------------

    #[test]
    fn rank_adjusted_exact_cutoff_returns_that_rank() {
        let interpolated = interp(Taxon::Bacteria, &BACTERIA_RANKS);

        assert_eq!(
            interpolated.get_rank_adjusted_by_identity(99.0),
            Some(DefaultRank(Species, 99.0))
        );
    }

    // BUG-R01: returns the deepest rank whose cutoff <= identity (FR-IDENT-03).
    #[test]
    fn rank_adjusted_by_identity_returns_deepest_satisfied_rank() {
        let interpolated = interp(Taxon::Bacteria, &BACTERIA_RANKS);

        assert_eq!(
            interpolated.get_rank_adjusted_by_identity(98.0),
            Some(DefaultRank(Genus, 97.0))
        );
        assert_eq!(
            interpolated.get_rank_adjusted_by_identity(99.0),
            Some(DefaultRank(Species, 99.0))
        );
        assert_eq!(
            interpolated.get_rank_adjusted_by_identity(99.5),
            Some(DefaultRank(Species, 99.0))
        );
        assert_eq!(interpolated.get_rank_adjusted_by_identity(50.0), None);
    }

    #[test]
    fn rank_adjusted_empty_interpolation_returns_none() {
        let interpolated =
            InterpolatedIdentity::new(Taxon::Bacteria, vec![], None).unwrap();

        assert_eq!(interpolated.get_rank_adjusted_by_identity(99.0), None);
    }

    // ---------------------------------------------------------------------
    // get_adjusted_taxonomy_by_identity
    // ---------------------------------------------------------------------

    #[test]
    fn adjusted_taxonomy_between_cutoffs_truncates_to_satisfied_ranks() {
        let interpolated = interp(Taxon::Bacteria, &BACTERIA_RANKS);

        let adjusted = interpolated
            .get_adjusted_taxonomy_by_identity(98.0, bacteria_lineage());

        assert_eq!(adjusted.len(), 6);
        assert_eq!(
            ranks_of(&adjusted),
            vec![Domain, Phylum, Class, Order, Family, Genus]
        );
    }

    #[test]
    fn adjusted_taxonomy_full_identity_keeps_all_ranks() {
        let interpolated = interp(Taxon::Bacteria, &BACTERIA_RANKS);

        let adjusted = interpolated
            .get_adjusted_taxonomy_by_identity(100.0, bacteria_lineage());

        assert_eq!(adjusted, bacteria_lineage());
    }

    #[test]
    fn adjusted_taxonomy_below_domain_cutoff_is_empty() {
        let interpolated = interp(Taxon::Bacteria, &BACTERIA_RANKS);

        let adjusted = interpolated
            .get_adjusted_taxonomy_by_identity(59.0, bacteria_lineage());

        assert!(adjusted.is_empty());
    }

    #[test]
    fn adjusted_taxonomy_non_monotonic_custom_keeps_zero_cutoff_ranks() {
        // Custom genus is None -> cutoff 0, so genus is kept while the shallower
        // domain (60) is discarded.
        let interpolated = InterpolatedIdentity::new(
            Taxon::Custom,
            ranks(&["d", "g", "s"]),
            Some(custom(r#"{"domain":60,"species":99}"#)),
        )
        .unwrap();
        let lineage = vec![
            bean(Domain, "bacteria"),
            bean(Genus, "bacillus"),
            bean(Species, "subtilis"),
        ];

        let adjusted =
            interpolated.get_adjusted_taxonomy_by_identity(50.0, lineage);

        assert_eq!(ranks_of(&adjusted), vec![Genus]);
    }

    #[test]
    fn adjusted_taxonomy_shorter_lineage_is_truncated_by_zip() {
        let interpolated = interp(Taxon::Bacteria, &BACTERIA_RANKS);
        let lineage =
            bacteria_lineage().into_iter().take(3).collect::<Vec<_>>();

        let adjusted = interpolated
            .get_adjusted_taxonomy_by_identity(100.0, lineage.clone());

        assert_eq!(adjusted, lineage);
    }

    #[test]
    fn adjusted_taxonomy_shorter_interpolation_is_truncated_by_zip() {
        let interpolated = interp(Taxon::Bacteria, &["d", "p"]);

        let adjusted = interpolated
            .get_adjusted_taxonomy_by_identity(100.0, bacteria_lineage());

        assert_eq!(ranks_of(&adjusted), vec![Domain, Phylum]);
    }
}
