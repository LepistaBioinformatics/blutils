use crate::domain::dtos::blast_result::{TaxonomyElement, TaxonomyFieldEnum};

pub(super) fn force_parsed_taxonomy(
    taxonomy: TaxonomyFieldEnum,
) -> Vec<TaxonomyElement> {
    match taxonomy {
        TaxonomyFieldEnum::Literal(_) => {
            panic!("Invalid format taxonomic field.")
        }
        TaxonomyFieldEnum::Parsed(res) => res,
    }
}
