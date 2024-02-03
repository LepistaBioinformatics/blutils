use crate::domain::dtos::taxonomy::TaxonomyBean;

pub(super) fn get_taxonomy_from_position(
    position: usize,
    taxonomy: Vec<TaxonomyBean>,
) -> Vec<TaxonomyBean> {
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
        .collect::<Vec<TaxonomyBean>>()
}
