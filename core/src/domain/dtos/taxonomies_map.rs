use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaxonomiesMap {
    pub(crate) blutils_version: String,
    pub(crate) ignore_taxids: Option<Vec<u64>>,
    pub(crate) replace_rank: Option<HashMap<String, String>>,
    pub(crate) drop_non_linnaean_taxonomies: Option<bool>,
    pub(crate) source_database: String,
    pub(crate) taxonomies: Vec<TaxonomyMapUnit>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Accession {
    pub(crate) accession: String,

    /// OID becomes the sequence original ID from the blast database
    pub(crate) oid: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaxonomyMapUnit {
    pub(crate) taxid: u64,
    pub(crate) rank: String,
    pub(crate) numeric_lineage: String,
    pub(crate) text_lineage: String,
    pub(crate) accessions: Vec<Accession>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taxonomies_map_serde_round_trip_uses_camel_case() {
        let json = serde_json::json!({
            "blutilsVersion": "8.3.1",
            "ignoreTaxids": [1, 2],
            "replaceRank": {"superkingdom": "domain"},
            "dropNonLinnaeanTaxonomies": true,
            "sourceDatabase": "ncbi",
            "taxonomies": [{
                "taxid": 561,
                "rank": "genus",
                "numericLineage": "d__2;g__561",
                "textLineage": "d__bacteria;g__escherichia",
                "accessions": [{"accession": "NR_1.1", "oid": "0"}]
            }]
        });

        let map: TaxonomiesMap = serde_json::from_value(json.clone()).unwrap();

        assert_eq!(map.taxonomies[0].taxid, 561);
        assert_eq!(map.taxonomies[0].accessions[0].oid, "0");
        assert_eq!(map.drop_non_linnaean_taxonomies, Some(true));
        assert_eq!(serde_json::to_value(&map).unwrap(), json);
    }

    #[test]
    fn taxonomies_map_optional_fields_missing_deserialize_as_none() {
        let map: TaxonomiesMap = serde_json::from_str(
            r#"{"blutilsVersion":"8.3.1","sourceDatabase":"ncbi","taxonomies":[]}"#,
        )
        .unwrap();

        assert!(map.ignore_taxids.is_none());
        assert!(map.replace_rank.is_none());
        assert!(map.drop_non_linnaean_taxonomies.is_none());
        assert!(map.taxonomies.is_empty());
    }

    #[test]
    fn taxonomies_map_snake_case_keys_rejected() {
        assert!(serde_json::from_str::<TaxonomiesMap>(
            r#"{"blutils_version":"8.3.1","source_database":"ncbi","taxonomies":[]}"#,
        )
        .is_err());
    }
}
