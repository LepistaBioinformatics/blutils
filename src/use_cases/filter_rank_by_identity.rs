use crate::domain::dtos::{
    blast_builder::Taxon::{self, *},
    blast_result::ValidTaxonomicRanksEnum,
};

use clean_base::utils::errors::MappedErrors;

/// Filter taxonomic rank by sequences identity percentage
///
/// The filtration process should be based in different identity cutoff points.
/// This, be careful on set the taxon parameter.
pub(crate) fn filter_rank_by_identity(
    taxon: Taxon,
    perc_identity: f64,
) -> Result<ValidTaxonomicRanksEnum, MappedErrors> {
    match taxon {
        Fungi => filter_fungi_identities(perc_identity),
        Bacteria => filter_bacteria_identities(perc_identity),
        Eukaryotes => filter_eukaryote_identities(perc_identity),
    }
}

/// Filter fungi ranks by identity percentage
///
/// TODO: Review the identity percentages and check a reference.
fn filter_fungi_identities(
    perc_identity: f64,
) -> Result<ValidTaxonomicRanksEnum, MappedErrors> {
    match perc_identity {
        i if i >= 97.0 => return Ok(ValidTaxonomicRanksEnum::Species),
        i if i >= 95.0 => return Ok(ValidTaxonomicRanksEnum::Genus),
        i if i >= 90.0 => return Ok(ValidTaxonomicRanksEnum::Family),
        i if i >= 85.0 => return Ok(ValidTaxonomicRanksEnum::Order),
        i if i >= 80.0 => return Ok(ValidTaxonomicRanksEnum::Class),
        i if i >= 75.0 => return Ok(ValidTaxonomicRanksEnum::Phylum),
        i if i >= 60.0 => return Ok(ValidTaxonomicRanksEnum::Domain),
        _ => return Ok(ValidTaxonomicRanksEnum::Undefined),
    };
}

/// Filter bacteria ranks by identity percentage
///
/// TODO: Review the identity percentages and check a reference.
fn filter_bacteria_identities(
    perc_identity: f64,
) -> Result<ValidTaxonomicRanksEnum, MappedErrors> {
    match perc_identity {
        i if i >= 99.0 => return Ok(ValidTaxonomicRanksEnum::Species),
        i if i >= 97.0 => return Ok(ValidTaxonomicRanksEnum::Genus),
        i if i >= 92.0 => return Ok(ValidTaxonomicRanksEnum::Family),
        i if i >= 85.0 => return Ok(ValidTaxonomicRanksEnum::Order),
        i if i >= 80.0 => return Ok(ValidTaxonomicRanksEnum::Class),
        i if i >= 75.0 => return Ok(ValidTaxonomicRanksEnum::Phylum),
        i if i >= 60.0 => return Ok(ValidTaxonomicRanksEnum::Domain),
        _ => return Ok(ValidTaxonomicRanksEnum::Undefined),
    };
}

/// Filter general eukaryotes ranks by identity percentage
///
/// TODO: Review the identity percentages and check a reference.
fn filter_eukaryote_identities(
    perc_identity: f64,
) -> Result<ValidTaxonomicRanksEnum, MappedErrors> {
    match perc_identity {
        i if i >= 97.0 => return Ok(ValidTaxonomicRanksEnum::Species),
        i if i >= 95.0 => return Ok(ValidTaxonomicRanksEnum::Genus),
        i if i >= 90.0 => return Ok(ValidTaxonomicRanksEnum::Family),
        i if i >= 85.0 => return Ok(ValidTaxonomicRanksEnum::Order),
        i if i >= 80.0 => return Ok(ValidTaxonomicRanksEnum::Class),
        i if i >= 75.0 => return Ok(ValidTaxonomicRanksEnum::Phylum),
        i if i >= 60.0 => return Ok(ValidTaxonomicRanksEnum::Domain),
        _ => return Ok(ValidTaxonomicRanksEnum::Undefined),
    };
}
