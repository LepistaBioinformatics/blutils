use self::Taxon::*;
use super::linnaean_ranks::{
    LinnaeanRank::*,
    RankedLinnaeanIdentity::{self, *},
};

use clap::ValueEnum;
use mycelium_base::utils::errors::{invalid_arg_err, MappedErrors};
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
    /// Loads custom cutoffs from a YAML (`.yaml`/`.yml`) or JSON (`.json`) file.
    pub fn from_file(path: PathBuf) -> Result<Self, MappedErrors> {
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .ok_or_else(|| {
                invalid_arg_err(format!(
                    "Custom taxon file must have an extension: {path:?}"
                ))
            })?
            .to_lowercase();

        if !matches!(extension.as_str(), "yaml" | "yml" | "json") {
            return invalid_arg_err(
                "Custom taxon file must be a YAML or JSON file",
            )
            .as_error();
        }

        let reader = std::fs::File::open(&path).map_err(|err| {
            invalid_arg_err(format!("Could not open custom taxon file: {err}"))
        })?;

        if extension == "json" {
            return serde_json::from_reader(&reader).map_err(|err| {
                invalid_arg_err(format!(
                    "Could not parse custom taxon file from JSON: {err}"
                ))
            });
        }

        serde_yaml_ng::from_reader(&reader).map_err(|err| {
            invalid_arg_err(format!(
                "Could not parse custom taxon file from YAML: {err}"
            ))
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "camelCase")]
pub enum Taxon {
    /// Fungi cutoff values based on default Internal Transcribed Spacer (ITS)
    /// cutoffs.
    ///
    #[value(alias = "f")]
    Fungi,

    /// Bacteria cutoff values are based on default 16S rRNA cutoffs.
    ///
    #[value(alias = "b")]
    Bacteria,

    /// Eukaryotes cutoff values are based on fungal Internal Transcribed Spacer
    /// (ITS) cutoffs.
    ///
    #[value(alias = "e")]
    Eukaryotes,

    /// Custom values must be provided by the user.
    ///
    #[value(alias = "c")]
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
            // BUG-R17: Kingdom uses the midpoint between Domain and Phylum.
            DefaultRank(Kingdom, 67.5),
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
            // BUG-R17: Kingdom uses the midpoint between Domain and Phylum.
            DefaultRank(Kingdom, 67.5),
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
            // BUG-R17: Kingdom uses the midpoint between Domain and Phylum.
            DefaultRank(Kingdom, 67.5),
            DefaultRank(Domain, 60.0),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_file(
        dir: &tempfile::TempDir,
        name: &str,
        content: &str,
    ) -> PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, content).unwrap();
        path
    }

    fn custom(domain: i16, species: i16) -> CustomTaxon {
        CustomTaxon {
            domain,
            kingdom: None,
            phylum: None,
            class: None,
            order: None,
            family: None,
            genus: None,
            species,
        }
    }

    // Pins the built-in values marked `TODO: review` (CONCERN-02).
    #[test]
    fn get_taxon_cutoff_bacteria_returns_pinned_table_species_first() {
        assert_eq!(
            Bacteria.get_taxon_cutoff(None),
            vec![
                DefaultRank(Species, 99.0),
                DefaultRank(Genus, 97.0),
                DefaultRank(Family, 92.0),
                DefaultRank(Order, 85.0),
                DefaultRank(Class, 80.0),
                DefaultRank(Phylum, 75.0),
                // BUG-R17
                DefaultRank(Kingdom, 67.5),
                DefaultRank(Domain, 60.0),
            ]
        );
    }

    #[test]
    fn get_taxon_cutoff_fungi_returns_pinned_table_species_first() {
        assert_eq!(
            Fungi.get_taxon_cutoff(None),
            vec![
                DefaultRank(Species, 97.0),
                DefaultRank(Genus, 95.0),
                DefaultRank(Family, 90.0),
                DefaultRank(Order, 85.0),
                DefaultRank(Class, 80.0),
                DefaultRank(Phylum, 75.0),
                // BUG-R17
                DefaultRank(Kingdom, 67.5),
                DefaultRank(Domain, 60.0),
            ]
        );
    }

    #[test]
    fn get_taxon_cutoff_eukaryotes_returns_same_table_as_fungi() {
        assert_eq!(
            Eukaryotes.get_taxon_cutoff(None),
            Fungi.get_taxon_cutoff(None)
        );
    }

    #[test]
    fn get_taxon_cutoff_builtin_ignores_custom_values() {
        assert_eq!(
            Bacteria.get_taxon_cutoff(Some(custom(1, 2))),
            Bacteria.get_taxon_cutoff(None)
        );
    }

    // BUG-R17: every built-in backbone has Kingdom halfway between Domain
    // (60) and Phylum (75).
    #[test]
    fn get_taxon_cutoff_builtin_has_kingdom_between_domain_and_phylum() {
        for taxon in [Bacteria, Fungi, Eukaryotes] {
            assert!(taxon
                .get_taxon_cutoff(None)
                .contains(&DefaultRank(Kingdom, 67.5)));
        }
    }

    // Custom tables run Domain -> Species (inverse of the built-ins) and
    // include Kingdom; missing optional ranks become 0.
    #[test]
    fn get_taxon_cutoff_custom_partial_values_fills_zeros_domain_first() {
        assert_eq!(
            Custom.get_taxon_cutoff(Some(custom(60, 99))),
            vec![
                DefaultRank(Domain, 60.0),
                DefaultRank(Kingdom, 0.0),
                DefaultRank(Phylum, 0.0),
                DefaultRank(Class, 0.0),
                DefaultRank(Order, 0.0),
                DefaultRank(Family, 0.0),
                DefaultRank(Genus, 0.0),
                DefaultRank(Species, 99.0),
            ]
        );
    }

    #[test]
    #[should_panic(expected = "Custom taxon values are required")]
    fn get_taxon_cutoff_custom_without_values_panics() {
        Custom.get_taxon_cutoff(None);
    }

    #[test]
    fn taxon_from_str_accepted_spellings_parse_to_variant() {
        for input in ["b", "Bacteria", "bacteria"] {
            assert!(matches!(input.parse::<Taxon>(), Ok(Bacteria)));
        }
        for input in ["f", "Fungi", "fungi"] {
            assert!(matches!(input.parse::<Taxon>(), Ok(Fungi)));
        }
        for input in ["e", "Eukaryotes", "eukaryotes"] {
            assert!(matches!(input.parse::<Taxon>(), Ok(Eukaryotes)));
        }
        for input in ["c", "Custom", "custom"] {
            assert!(matches!(input.parse::<Taxon>(), Ok(Custom)));
        }
    }

    #[test]
    fn taxon_from_str_uppercase_returns_err() {
        assert!(matches!("BACTERIA".parse::<Taxon>(), Err(())));
        assert!("".parse::<Taxon>().is_err());
    }

    #[test]
    fn taxon_serde_uses_lowercase_names() {
        for (taxon, name) in [
            (Bacteria, "\"bacteria\""),
            (Fungi, "\"fungi\""),
            (Eukaryotes, "\"eukaryotes\""),
            (Custom, "\"custom\""),
        ] {
            assert_eq!(serde_json::to_string(&taxon).unwrap(), name);
            let back: Taxon = serde_json::from_str(name).unwrap();
            assert_eq!(serde_json::to_string(&back).unwrap(), name);
        }
    }

    #[test]
    fn taxon_deserialize_capitalized_name_fails() {
        assert!(serde_json::from_str::<Taxon>("\"Bacteria\"").is_err());
    }

    #[test]
    fn custom_taxon_from_file_yaml_loads_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(
            &dir,
            "taxon.yaml",
            "domain: 60\ngenus: 95\nspecies: 99\n",
        );

        let taxon = CustomTaxon::from_file(path).unwrap();

        assert_eq!(
            Custom.get_taxon_cutoff(Some(taxon)),
            vec![
                DefaultRank(Domain, 60.0),
                DefaultRank(Kingdom, 0.0),
                DefaultRank(Phylum, 0.0),
                DefaultRank(Class, 0.0),
                DefaultRank(Order, 0.0),
                DefaultRank(Family, 0.0),
                DefaultRank(Genus, 95.0),
                DefaultRank(Species, 99.0),
            ]
        );
    }

    // The shipped example cutoff file keeps loading (serde_yaml_ng swap).
    #[test]
    fn custom_taxon_from_file_bacteria_16s_yaml_asset_loads_values() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../assets/custom-taxon-cutoffs-bacteria-16S.yaml");

        let taxon = CustomTaxon::from_file(path).unwrap();

        assert_eq!(
            Custom.get_taxon_cutoff(Some(taxon)),
            vec![
                DefaultRank(Domain, 50.0),
                DefaultRank(Kingdom, 60.0),
                DefaultRank(Phylum, 75.0),
                DefaultRank(Class, 80.0),
                DefaultRank(Order, 85.0),
                DefaultRank(Family, 92.0),
                DefaultRank(Genus, 97.0),
                DefaultRank(Species, 99.0),
            ]
        );
    }

    #[test]
    fn custom_taxon_from_file_json_loads_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(
            &dir,
            "taxon.json",
            r#"{"domain": 70, "kingdom": 72, "species": 98}"#,
        );

        let taxon = CustomTaxon::from_file(path).unwrap();

        assert_eq!(taxon.domain, 70);
        assert_eq!(taxon.kingdom, Some(72));
        assert_eq!(taxon.phylum, None);
        assert_eq!(taxon.species, 98);
    }

    fn from_file_error(path: PathBuf) -> String {
        CustomTaxon::from_file(path)
            .expect_err("loading should fail")
            .to_string()
    }

    // BUG-R19 regression: every error path returns an error instead of panicking.
    #[test]
    fn custom_taxon_from_file_without_extension_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(&dir, "taxon", "domain: 60\nspecies: 99\n");
        assert!(from_file_error(path).contains("must have an extension"));
    }

    #[test]
    fn custom_taxon_from_file_unsupported_extension_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(&dir, "taxon.toml", "domain = 60\n");
        assert!(from_file_error(path).contains("must be a YAML or JSON file"));
    }

    // BUG-R19 regression: `.yml` is accepted as YAML.
    #[test]
    fn custom_taxon_from_file_yml_extension_loads_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(&dir, "taxon.yml", "domain: 60\nspecies: 99\n");
        let taxon = CustomTaxon::from_file(path).unwrap();
        assert_eq!(taxon.domain, 60);
        assert_eq!(taxon.species, 99);
    }

    // BUG-R19 regression
    #[test]
    fn custom_taxon_from_file_nonexistent_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let message = from_file_error(dir.path().join("missing.yaml"));
        assert!(message.contains("Could not open custom taxon file"));
    }

    #[test]
    fn custom_taxon_from_file_invalid_yaml_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(&dir, "taxon.yaml", "domain: [unclosed\n");
        assert!(from_file_error(path)
            .contains("Could not parse custom taxon file from YAML"));
    }

    #[test]
    fn custom_taxon_from_file_invalid_json_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(&dir, "taxon.json", "{\"domain\": ");
        assert!(from_file_error(path)
            .contains("Could not parse custom taxon file from JSON"));
    }

    // Fields are i16: fractional cutoffs are rejected at parse time.
    #[test]
    fn custom_taxon_from_file_fractional_value_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let path =
            write_file(&dir, "taxon.yaml", "domain: 60\nspecies: 97.5\n");
        assert!(from_file_error(path)
            .contains("Could not parse custom taxon file from YAML"));
    }

    #[test]
    fn custom_taxon_from_file_missing_species_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(&dir, "taxon.json", r#"{"domain": 60}"#);
        assert!(from_file_error(path).contains("missing field `species`"));
    }
}
