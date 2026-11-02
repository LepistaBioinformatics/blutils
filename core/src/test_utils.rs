//! Builders shared by in-crate unit tests. Compiled only under `cfg(test)`.

use crate::domain::dtos::{
    blast_result::BlastResultRow,
    linnaean_ranks::{InterpolatedIdentity, LinnaeanRank},
    taxon::Taxon,
    taxonomy_bean::Taxonomy,
};
use std::str::FromStr;

/// A BLAST hit whose taxonomy is still an unparsed literal lineage
/// (e.g. `"d__bacteria;g__bacillus"`).
pub(crate) fn row(
    accession: &str,
    taxid: i64,
    perc_identity: f64,
    bit_score: i64,
    lineage: &str,
) -> BlastResultRow {
    BlastResultRow {
        subject_accession: accession.to_string(),
        subject_taxid: taxid,
        perc_identity,
        align_length: 500,
        mismatches: 0,
        gap_openings: 0,
        q_start: 1,
        q_end: 500,
        s_start: 1,
        s_end: 500,
        e_value: 1e-100,
        bit_score: bit_score as f64,
        taxonomy: Taxonomy::Literal(lineage.to_string()),
    }
}

/// Parses rank tokens (`"d"`, `"species-group"`, ...) into `LinnaeanRank`s.
pub(crate) fn ranks(tokens: &[&str]) -> Vec<LinnaeanRank> {
    tokens
        .iter()
        .map(|token| LinnaeanRank::from_str(token).unwrap())
        .collect()
}

/// Interpolated identities for a built-in taxon over the given rank tokens.
pub(crate) fn interp(taxon: Taxon, tokens: &[&str]) -> InterpolatedIdentity {
    InterpolatedIdentity::new(taxon, ranks(tokens), None).unwrap()
}
