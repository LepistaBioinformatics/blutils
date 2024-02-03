use crate::domain::dtos::taxonomy::TaxonomyBean;

/// Get lowest Blast statistics of a single rank vector
///
/// Collect the lowest statistics guarantees that the worst case will be
/// selected, avoiding over-interpretation of the results.
pub(super) fn get_rank_lowest_statistics(
    mut rank_taxonomies: Vec<TaxonomyBean>,
) -> TaxonomyBean {
    rank_taxonomies
        .sort_by(|a, b| a.perc_identity.partial_cmp(&b.perc_identity).unwrap());

    rank_taxonomies.first().unwrap().to_owned()
}
