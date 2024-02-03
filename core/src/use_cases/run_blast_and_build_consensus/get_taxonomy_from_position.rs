use crate::domain::dtos::blast_result::TaxonomyElement;

pub(super) fn get_taxonomy_from_position(
    position: usize,
    taxonomy: Vec<TaxonomyElement>,
) -> Vec<TaxonomyElement> {
    taxonomy
        .into_iter()
        .enumerate()
        .filter_map(
            |(index, rank)| {
                if index <= position {
                    Some(rank)
                } else {
                    None
                }
            },
        )
        .collect::<Vec<TaxonomyElement>>()
}
