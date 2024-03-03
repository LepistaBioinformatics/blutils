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
