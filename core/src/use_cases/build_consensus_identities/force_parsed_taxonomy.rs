use crate::domain::dtos::taxonomy_bean::{Taxonomy, TaxonomyBean};

pub(super) fn force_parsed_taxonomy(taxonomy: Taxonomy) -> Vec<TaxonomyBean> {
    match taxonomy {
        Taxonomy::Literal(_) => {
            panic!("Invalid format taxonomic field.")
        }
        Taxonomy::Parsed(res) => res,
    }
}
