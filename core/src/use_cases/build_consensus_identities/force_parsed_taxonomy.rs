use crate::domain::dtos::taxonomy_bean::{Taxonomy, TaxonomyBean};

use mycelium_base::utils::errors::{use_case_err, MappedErrors};

/// BUG-R43: an unparsed lineage returns `Err` instead of panicking.
pub(super) fn force_parsed_taxonomy(
    taxonomy: Taxonomy,
) -> Result<Vec<TaxonomyBean>, MappedErrors> {
    match taxonomy {
        Taxonomy::Literal(lineage) => use_case_err(format!(
            "Lineage was not parsed before building the consensus: {lineage:?}"
        ))
        .as_error(),
        Taxonomy::Parsed(res) => Ok(res),
    }
}
