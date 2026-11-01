use super::{
    load_del_nodes_dataframe, load_lineage_dataframe, load_merged_dataframe,
    load_names_dataframe, load_nodes_dataframe,
};
use crate::{
    domain::dtos::{
        linnaean_ranks::LinnaeanRank,
        taxonomies_map::{Accession, TaxonomiesMap, TaxonomyMapUnit},
    },
    use_cases::shared::write_or_append_to_file,
};

use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use polars_ops::{
    frame::{JoinArgs, JoinType},
    prelude::DataFrameJoinOps,
};
use slugify::slugify;
use std::{
    collections::HashMap,
    fs::{remove_file, File},
    io::Write,
    path::PathBuf,
    str::FromStr,
};
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub(crate) struct RankedTaxidUnit {
    pub name: String,
    pub rank: String,
    pub lineage: String,
}

#[tracing::instrument(
    name = "Build Taxonomy DB",
    skip(
        names_path,
        nodes_path,
        lineage_path,
        del_nodes_path,
        merged_path,
        accessions_map,
        ignore_taxids,
        replace_rank,
        drop_non_linnaean_taxonomies,
    )
)]
pub(crate) fn build_taxonomy_database(
    names_path: PathBuf,
    nodes_path: PathBuf,
    lineage_path: PathBuf,
    del_nodes_path: PathBuf,
    merged_path: PathBuf,
    accessions_map: HashMap<u64, Vec<Accession>>,
    ignore_taxids: Option<Vec<u64>>,
    replace_rank: Option<HashMap<String, String>>,
    drop_non_linnaean_taxonomies: Option<bool>,
    database: String,
    output_file_path: PathBuf,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Validate arguments
    // ? -----------------------------------------------------------------------

    if !names_path.is_file() {
        return use_case_err(format!("Invalid names path: {:?}", names_path))
            .as_error();
    }

    if !lineage_path.is_file() {
        return use_case_err(format!(
            "Invalid lineages path: {:?}",
            lineage_path
        ))
        .as_error();
    }

    if !nodes_path.is_file() {
        return use_case_err(format!("Invalid nodes path: {:?}", nodes_path))
            .as_error();
    }

    if !del_nodes_path.is_file() {
        return use_case_err(format!(
            "Invalid delnodes path: {:?}",
            del_nodes_path
        ))
        .as_error();
    }

    if !merged_path.is_file() {
        return use_case_err(format!("Invalid merged path: {:?}", merged_path))
            .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Load reference data-frames
    // ? -----------------------------------------------------------------------

    debug!("Loading and validating `DELETED` nodes");
    let del_nodes_vector = load_del_nodes_dataframe(del_nodes_path)?;

    debug!("Loading and validating `MERGED` nodes");
    let merged_map = load_merged_dataframe(merged_path)?;

    debug!("Loading and validating `NAMES`");
    let names_df = load_names_dataframe(names_path)?;

    debug!("Loading and validating `NODES`");

    let nodes_df = load_nodes_dataframe(nodes_path)?;

    debug!("Loading and validating `LINEAGES`");

    let lineages_df = load_lineage_dataframe(lineage_path)?;

    debug!("Joining `NODES` and `LINEAGES`");

    let nodes_and_lineages_df = match nodes_df.join(
        &lineages_df,
        ["tax_id"],
        ["tax_id"],
        JoinArgs {
            how: JoinType::Inner,
            ..Default::default()
        },
    ) {
        Ok(df) => df,
        Err(err) => {
            return use_case_err(format!(
                "Unexpected error detected on join names and nodes: {err}"
            ))
            .as_error();
        }
    };

    debug!("Joining `NODES` and `LINEAGES` with `NAMES`");

    let nodes_and_lineages_with_names_df = match nodes_and_lineages_df.join(
        &names_df,
        ["tax_id"],
        ["tax_id"],
        JoinArgs {
            how: JoinType::Left,
            ..Default::default()
        },
    ) {
        Ok(df) => df,
        Err(err) => {
            return use_case_err(format!(
                "Unexpected error detected on join names, nodes, with lineages: {err}"
            ))
            .as_error();
        }
    };

    debug!("Reducing `NODES` and `LINEAGES` with `NAMES`");

    let reduced_df = match nodes_and_lineages_with_names_df.select([
        "tax_id",
        "rank",
        "text_name",
        "lineage",
    ]) {
        Ok(df) => df,
        Err(err) => {
            return use_case_err(format!(
                "Unexpected error detected build taxonomies dataframe: {err}"
            ))
            .as_error();
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Fold taxonomies
    // ? -----------------------------------------------------------------------

    debug!("Building fully qualified taxonomies");

    let binding = reduced_df.clone();
    let mut iters = binding
        .columns(["tax_id", "rank", "text_name", "lineage"])
        .unwrap()
        .iter()
        .map(|s| s.iter())
        .collect::<Vec<_>>();

    let mut ranked_tax_ids: HashMap<u64, RankedTaxidUnit> = HashMap::new();
    for _ in 0..binding.height() {
        let tax_id = iters[0]
            .next()
            .unwrap()
            .to_string()
            .trim()
            .parse::<i32>()
            .unwrap() as u64;

        let rank = iters[1]
            .next()
            .unwrap()
            .to_string()
            .trim()
            .to_string()
            .replace("\"", "")
            .to_lowercase();

        let name = iters[2]
            .next()
            .unwrap()
            .to_string()
            .trim()
            .to_string()
            .replace("\"", "");

        let lineage = iters[3]
            .next()
            .unwrap()
            .to_string()
            .trim()
            .to_string()
            .replace("\"", "");

        ranked_tax_ids.insert(
            tax_id,
            RankedTaxidUnit {
                rank,
                name: match name {
                    name if name.is_empty() || name == "null" => {
                        format!("taxid-{tax_id}")
                    }
                    name => name,
                },
                lineage,
            },
        );
    }

    // ? -----------------------------------------------------------------------
    // ? Build output files
    // ? -----------------------------------------------------------------------

    let mut output_path = output_file_path;
    output_path.set_extension("json");

    //
    // Create the main output file path
    //
    let output_database_file = match output_path.parent() {
        Some(parent) => parent.join(format!(
            "{}.blutils.json",
            output_path.file_stem().unwrap().to_str().unwrap()
        )),
        None => PathBuf::from("blutils.json"),
    };

    //
    // Create a file to include not mapped tax_ids
    //
    let non_mapped_file = match output_path.parent() {
        Some(parent) => parent.join(format!(
            "{}.non-mapped.tsv",
            output_path.file_stem().unwrap().to_str().unwrap()
        )),
        None => PathBuf::from("non-mapped.tsv"),
    };

    let non_mapped_file_file_binding = non_mapped_file.as_path();

    if non_mapped_file_file_binding.exists() {
        remove_file(non_mapped_file_file_binding).unwrap();
    }

    let (non_mapped_writer, non_mapped_file) =
        write_or_append_to_file(&non_mapped_file_file_binding);

    // ? -----------------------------------------------------------------------
    // ? Hydrate lineages and Build the output taxonomies dataframe
    // ? -----------------------------------------------------------------------

    let mut taxonomies = Vec::<TaxonomyMapUnit>::new();

    accessions_map.into_iter().for_each(|(tax_id, accessions)| {
        let header = format!("{tax_id}");

        //
        // `leaf_tax_id` is the taxid whose record builds the lineages. It
        // differs from `tax_id` only for merged nodes, where it is the merge
        // target, so the numeric leaf agrees with the text leaf (BUG-R31).
        //
        let (leaf_tax_id, ranked_tax_id) = match ranked_tax_ids.get(&tax_id) {
            Some(res) => (tax_id, res),
            None => {
                //
                // This condition is triggered when a tax_id is not found in the
                // taxdump files and is a deleted node.
                //
                if del_nodes_vector.contains(&tax_id) {
                    match non_mapped_writer(
                        format!("{}\t{}\n", header, "deleted"),
                        non_mapped_file
                            .try_clone()
                            .expect("Unexpected error detected on write non-mapped file"),
                    ) {
                        Ok(_) => (),
                        Err(err) => panic!("{err}")
                    };

                    return;
                }

                //
                // This condition is triggered when a tax_id is not found in the
                // taxdump files and is a merged node.
                //
                if let Some(new_tax_id) = merged_map.get(&tax_id) {
                    match ranked_tax_ids.get(&new_tax_id) {
                        Some(res) => (*new_tax_id, res),
                        None => {

                            match non_mapped_writer(
                                format!("{}\t{}\n", header, "merged"),
                                non_mapped_file
                                    .try_clone()
                                    .expect("Unexpected error detected on write non-mapped file"),
                            ) {
                                Ok(_) => (),
                                Err(err) => panic!("{err}")
                            };

                            return;
                        }
                    }
                } else {
                    match non_mapped_writer(
                        format!("{}\t{}\n", header, "unknown"),
                        non_mapped_file
                            .try_clone()
                            .expect("Unexpected error detected on write non-mapped file"),
                    ) {
                        Ok(_) => (),
                        Err(err) => panic!("{err}")
                    };

                    return;
                }
            }
        };

        let lineage = ranked_tax_id.lineage
            .split(" ")
            .flat_map(|lineage_tax_id| {
                if lineage_tax_id.is_empty() || lineage_tax_id == "null" {
                    return None;
                }

                let lineage_tax_id = lineage_tax_id.trim().parse::<u64>().unwrap();

                if let Some(taxids) = ignore_taxids.to_owned() {
                    if taxids.contains(&lineage_tax_id) {
                        return None;
                    }
                }

                let record = match ranked_tax_ids.get(&lineage_tax_id) {
                    Some(res) => res,
                    None => {
                        warn!(
                            "Unmapped tax_id detected {lineage_tax_id} in lineage: {lineage}",
                            lineage = ranked_tax_id.lineage
                        );

                        return None;
                    }
                };

                let valid_rank = match replace_rank.to_owned() {
                    Some(replace_rank) => {
                        if let Some(replaced_rank) = replace_rank.get(&record.rank) {
                            replaced_rank.to_string()
                        } else {
                            record.rank.to_string()
                        }
                    }
                    None => record.rank.to_string(),
                };

                let valid_rank = match valid_rank.parse::<LinnaeanRank>() {
                    Ok(res) => match res {
                        //
                        // Skip non linnaean taxonomies if the non-linnaean rank
                        // was found and the `drop_non_linnaean_taxonomies` flag
                        // is set to true.
                        //
                        LinnaeanRank::Other(rank) => {
                            if let Some(true) = drop_non_linnaean_taxonomies {
                                return None;
                            } else {
                                slugify!(rank.clone().as_str(), separator = "-")
                            }
                        }
                        _ => res.to_string(),
                    },
                    Err(_) => {
                        panic!(
                            "Unexpected error detected on parse rank: {}", record.rank
                        )
                    },
                };

                let ranked_name = format!(
                    "{}__{}",
                    valid_rank,
                    slugify!(record.name.as_str())
                );

                let ranked_taxid = format!(
                    "{}__{}",
                    valid_rank,
                    lineage_tax_id
                );

                Some((ranked_taxid, ranked_name))
            })
            .collect::<Vec<(String, String)>>();

        //
        // Skip non linnaean taxonomies if the non-linnaean rank was found and
        // the `drop_non_linnaean_taxonomies` flag is set to true.
        //
        // Apply `replace_rank` to the leaf as done for the ancestors
        // (BUG-R30).
        let leaf_rank = replace_rank
            .as_ref()
            .and_then(|replace_rank| replace_rank.get(&ranked_tax_id.rank))
            .unwrap_or(&ranked_tax_id.rank);

        let slug_rank = match LinnaeanRank::from_str(leaf_rank) {
            Ok(res) => match res {
                LinnaeanRank::Other(rank) => {
                    if let Some(true) = drop_non_linnaean_taxonomies {
                        return;
                    } else {
                        slugify!(rank.as_str(), separator = "-")
                    }
                }
                _ => res.to_string(),
            },
            Err(_) => slugify!(leaf_rank.as_str(), separator = "-"),
        };

        //
        // Write the taxi-ds based taxonomies to the output files
        //
        // The leaf is appended as one more element, so an empty lineage never
        // yields a leading `;` (BUG-R16).
        let ranked_taxids = lineage
            .iter()
            .map(|(ranked_taxid,_)| ranked_taxid.to_string())
            .chain(std::iter::once(format!("{}__{}", slug_rank, leaf_tax_id)))
            .collect::<Vec<String>>()
            .join(";");

        //
        // Write the names based taxonomies to the output files
        //
        let ranked_names = lineage
            .iter()
            .map(|(_, ranked_name)| ranked_name.to_string())
            .chain(std::iter::once(format!(
                "{}__{}",
                slug_rank,
                slugify!(ranked_tax_id.name.as_str())
            )))
            .collect::<Vec<String>>()
            .join(";");

        taxonomies.push(TaxonomyMapUnit {
            taxid: tax_id,
            rank: slug_rank,
            numeric_lineage: ranked_taxids,
            text_lineage: ranked_names,
            accessions: accessions.to_owned(),
        });
    });

    let mut file = match File::create(output_database_file) {
        Err(err) => panic!("{err}"),
        Ok(res) => res,
    };

    match file.write_all(
        serde_json::to_string_pretty(&TaxonomiesMap {
            blutils_version: env!("CARGO_PKG_VERSION").to_string(),
            ignore_taxids,
            replace_rank,
            drop_non_linnaean_taxonomies,
            source_database: database,
            taxonomies,
        })
        .unwrap()
        .as_bytes(),
    ) {
        Err(err) => panic!("{err}"),
        Ok(_) => (),
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::dtos::taxonomy_bean::Taxonomy, test_utils::row};
    use std::fs::{read_to_string, write};
    use tempfile::TempDir;

    // Tree used by every test:
    //
    //   1 root (no rank)
    //   └ 131567 cellular organisms (no rank)
    //     └ 2 Bacteria (superkingdom)
    //       └ 1783272 Terrabacteria group (clade)
    //         └ 1239 Bacillota (phylum)
    //           └ 1386 Bacillus (genus)
    //             ├ 1423 Bacillus subtilis (species)
    //             │ └ 950 Bacillus subtilis str. 168 (strain)
    //             ├ 900 <no names.dmp entry> (species)
    //             ├ 901 <empty scientific name> (species)
    //             └ 902 Odd__name (species)
    //   77 Orphan species (species) with an empty lineage
    //
    // 500 is deleted, 600 was merged into 1423, 700 was merged into 999
    // (absent from nodes.dmp) and 800 is unknown.
    const NAMES: &str = "\
1\t|\troot\t|\t\t|\tscientific name\t|
131567\t|\tcellular organisms\t|\t\t|\tscientific name\t|
2\t|\tBacteria\t|\tBacteria <bacteria>\t|\tscientific name\t|
2\t|\teubacteria\t|\t\t|\tgenbank common name\t|
1783272\t|\tTerrabacteria group\t|\t\t|\tscientific name\t|
1239\t|\tBacillota\t|\t\t|\tscientific name\t|
1386\t|\tBacillus\t|\t\t|\tscientific name\t|
1423\t|\tBacillus subtilis\t|\t\t|\tscientific name\t|
950\t|\tBacillus subtilis str. 168\t|\t\t|\tscientific name\t|
901\t|\t\t|\t\t|\tscientific name\t|
902\t|\tOdd__name\t|\t\t|\tscientific name\t|
77\t|\tOrphan species\t|\t\t|\tscientific name\t|
";

    const NODES: &str = "\
1\t|\t1\t|\tno rank\t|
131567\t|\t1\t|\tno rank\t|
2\t|\t131567\t|\tsuperkingdom\t|
1783272\t|\t2\t|\tclade\t|
1239\t|\t1783272\t|\tphylum\t|
1386\t|\t1239\t|\tgenus\t|
1423\t|\t1386\t|\tspecies\t|
950\t|\t1423\t|\tstrain\t|
900\t|\t1386\t|\tspecies\t|
901\t|\t1386\t|\tspecies\t|
902\t|\t1386\t|\tspecies\t|
77\t|\t1\t|\tspecies\t|
";

    const LINEAGES: &str = "\
1\t|\t\t|
131567\t|\t1 \t|
2\t|\t1 131567 \t|
1783272\t|\t1 131567 2 \t|
1239\t|\t1 131567 2 1783272 \t|
1386\t|\t1 131567 2 1783272 1239 \t|
1423\t|\t1 131567 2 1783272 1239 1386 \t|
950\t|\t1 131567 2 1783272 1239 1386 1423 \t|
900\t|\t1 131567 2 1783272 1239 1386 \t|
901\t|\t1 131567 2 1783272 1239 1386 \t|
902\t|\t1 131567 2 1783272 1239 1386 \t|
77\t|\t\t|
";

    const DEL_NODES: &str = "500\t|\n";

    const MERGED: &str = "600\t|\t1423\t|\n700\t|\t999\t|\n";

    struct Taxdump {
        dir: TempDir,
    }

    impl Taxdump {
        fn new() -> Self {
            let taxdump = Taxdump {
                dir: tempfile::tempdir().unwrap(),
            };

            for (name, content) in [
                ("names.dmp", NAMES),
                ("nodes.dmp", NODES),
                ("taxidlineage.dmp", LINEAGES),
                ("delnodes.dmp", DEL_NODES),
                ("merged.dmp", MERGED),
            ] {
                write(taxdump.path(name), content).unwrap();
            }

            taxdump
        }

        fn path(&self, name: &str) -> PathBuf {
            self.dir.path().join(name)
        }

        fn build(
            &self,
            taxids: &[u64],
            ignore_taxids: Option<Vec<u64>>,
            replace_rank: Option<HashMap<String, String>>,
            drop_non_linnaean_taxonomies: Option<bool>,
        ) -> Result<(), MappedErrors> {
            build_taxonomy_database(
                self.path("names.dmp"),
                self.path("nodes.dmp"),
                self.path("taxidlineage.dmp"),
                self.path("delnodes.dmp"),
                self.path("merged.dmp"),
                accessions_map(taxids),
                ignore_taxids,
                replace_rank,
                drop_non_linnaean_taxonomies,
                "fixture-db".to_string(),
                self.path("out.whatever"),
            )
        }

        fn database(&self) -> TaxonomiesMap {
            let mut map: TaxonomiesMap = serde_json::from_str(
                &read_to_string(self.path("out.blutils.json")).unwrap(),
            )
            .unwrap();
            map.taxonomies.sort_by_key(|unit| unit.taxid);
            map
        }

        fn non_mapped(&self) -> Vec<String> {
            let mut lines = read_to_string(self.path("out.non-mapped.tsv"))
                .unwrap()
                .lines()
                .map(String::from)
                .collect::<Vec<_>>();
            lines.sort();
            lines
        }

        fn unit(&self, taxid: u64) -> TaxonomyMapUnit {
            self.database()
                .taxonomies
                .into_iter()
                .find(|unit| unit.taxid == taxid)
                .unwrap_or_else(|| panic!("taxid {taxid} not in database"))
        }
    }

    fn accessions_map(taxids: &[u64]) -> HashMap<u64, Vec<Accession>> {
        taxids
            .iter()
            .map(|taxid| {
                (
                    *taxid,
                    vec![Accession {
                        accession: format!("ACC{taxid}.1"),
                        oid: taxid.to_string(),
                    }],
                )
            })
            .collect()
    }

    fn lineages(unit: &TaxonomyMapUnit) -> (&str, &str) {
        (unit.numeric_lineage.as_str(), unit.text_lineage.as_str())
    }

    fn superkingdom_as_domain() -> Option<HashMap<String, String>> {
        Some(HashMap::from([(
            "superkingdom".to_string(),
            "d".to_string(),
        )]))
    }

    fn error_message(result: Result<(), MappedErrors>) -> String {
        format!("{}", result.unwrap_err())
    }

    fn assert_missing_file_error(file: &str, expected: &str) {
        let taxdump = Taxdump::new();
        std::fs::remove_file(taxdump.path(file)).unwrap();

        let message = error_message(taxdump.build(&[1423], None, None, None));

        assert!(message.contains(expected), "{message}");
        assert!(!taxdump.path("out.blutils.json").exists());
    }

    #[test]
    fn build_taxonomy_database_missing_names_returns_err() {
        assert_missing_file_error("names.dmp", "Invalid names path");
    }

    #[test]
    fn build_taxonomy_database_missing_nodes_returns_err() {
        assert_missing_file_error("nodes.dmp", "Invalid nodes path");
    }

    #[test]
    fn build_taxonomy_database_missing_lineage_returns_err() {
        assert_missing_file_error("taxidlineage.dmp", "Invalid lineages path");
    }

    #[test]
    fn build_taxonomy_database_missing_delnodes_returns_err() {
        assert_missing_file_error("delnodes.dmp", "Invalid delnodes path");
    }

    #[test]
    fn build_taxonomy_database_missing_merged_returns_err() {
        assert_missing_file_error("merged.dmp", "Invalid merged path");
    }

    #[test]
    fn build_taxonomy_database_output_path_writes_database_and_non_mapped_files(
    ) {
        let taxdump = Taxdump::new();

        taxdump.build(&[1423], None, None, None).unwrap();

        assert!(taxdump.path("out.blutils.json").is_file());
        assert!(!taxdump.path("out.whatever").exists());
        assert!(!taxdump.path("out.json").exists());
        assert_eq!(
            read_to_string(taxdump.path("out.non-mapped.tsv")).unwrap(),
            ""
        );
    }

    #[test]
    fn build_taxonomy_database_writes_metadata_and_accessions() {
        let taxdump = Taxdump::new();

        taxdump
            .build(
                &[1423],
                Some(vec![131567]),
                superkingdom_as_domain(),
                Some(true),
            )
            .unwrap();

        let database = taxdump.database();
        assert_eq!(database.blutils_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(database.source_database, "fixture-db");
        assert_eq!(database.ignore_taxids, Some(vec![131567]));
        assert_eq!(database.replace_rank, superkingdom_as_domain());
        assert_eq!(database.drop_non_linnaean_taxonomies, Some(true));
        assert_eq!(database.taxonomies.len(), 1);
        assert_eq!(database.taxonomies[0].accessions.len(), 1);
        assert_eq!(database.taxonomies[0].accessions[0].accession, "ACC1423.1");
        assert_eq!(database.taxonomies[0].accessions[0].oid, "1423");
    }

    #[test]
    fn build_taxonomy_database_unmapped_taxids_go_to_non_mapped_file() {
        let taxdump = Taxdump::new();

        taxdump
            .build(&[1423, 500, 700, 800], None, None, None)
            .unwrap();

        assert_eq!(
            taxdump.non_mapped(),
            vec!["500\tdeleted", "700\tmerged", "800\tunknown"]
        );
        assert_eq!(
            taxdump
                .database()
                .taxonomies
                .iter()
                .map(|unit| unit.taxid)
                .collect::<Vec<_>>(),
            vec![1423]
        );
    }

    #[test]
    fn build_taxonomy_database_default_options_keep_every_ancestor() {
        let taxdump = Taxdump::new();

        taxdump.build(&[1423], None, None, None).unwrap();

        let unit = taxdump.unit(1423);
        assert_eq!(unit.rank, "s");
        assert_eq!(
            lineages(&unit),
            (
                "no-rank__1;no-rank__131567;superkingdom__2;clade__1783272;\
                 p__1239;g__1386;s__1423",
                "no-rank__root;no-rank__cellular-organisms;superkingdom__bacteria;\
                 clade__terrabacteria-group;p__bacillota;g__bacillus;\
                 s__bacillus-subtilis"
            )
        );
    }

    #[test]
    fn build_taxonomy_database_replace_rank_and_drop_non_linnaean_yield_linnaean_lineage(
    ) {
        let taxdump = Taxdump::new();

        taxdump
            .build(&[1423], None, superkingdom_as_domain(), Some(true))
            .unwrap();

        assert_eq!(
            lineages(&taxdump.unit(1423)),
            (
                "d__2;p__1239;g__1386;s__1423",
                "d__bacteria;p__bacillota;g__bacillus;s__bacillus-subtilis"
            )
        );
    }

    // BUG-R30: `replace_rank` also applies to the leaf rank.
    #[test]
    fn build_taxonomy_database_replace_rank_applies_to_leaf_rank() {
        let taxdump = Taxdump::new();

        taxdump
            .build(&[2], None, superkingdom_as_domain(), Some(false))
            .unwrap();

        let unit = taxdump.unit(2);
        assert_eq!(unit.rank, "d");
        assert_eq!(
            lineages(&unit),
            (
                "no-rank__1;no-rank__131567;d__2",
                "no-rank__root;no-rank__cellular-organisms;d__bacteria"
            )
        );
    }

    #[test]
    fn build_taxonomy_database_drop_non_linnaean_discards_non_linnaean_leaf() {
        let taxdump = Taxdump::new();

        taxdump.build(&[1423, 950], None, None, Some(true)).unwrap();

        assert_eq!(
            taxdump
                .database()
                .taxonomies
                .iter()
                .map(|unit| unit.taxid)
                .collect::<Vec<_>>(),
            vec![1423]
        );
        assert!(taxdump.non_mapped().is_empty());
    }

    #[test]
    fn build_taxonomy_database_non_linnaean_leaf_kept_without_drop_flag() {
        let taxdump = Taxdump::new();

        taxdump.build(&[950], None, None, None).unwrap();

        let unit = taxdump.unit(950);
        assert_eq!(unit.rank, "strain");
        assert!(unit.numeric_lineage.ends_with(";s__1423;strain__950"));
        assert!(unit.text_lineage.ends_with(
            ";s__bacillus-subtilis;strain__bacillus-subtilis-str-168"
        ));
    }

    #[test]
    fn build_taxonomy_database_ignore_taxids_removes_ancestors() {
        let taxdump = Taxdump::new();

        taxdump
            .build(
                &[1423],
                Some(vec![1, 131567, 1783272]),
                superkingdom_as_domain(),
                None,
            )
            .unwrap();

        assert_eq!(
            lineages(&taxdump.unit(1423)),
            (
                "d__2;p__1239;g__1386;s__1423",
                "d__bacteria;p__bacillota;g__bacillus;s__bacillus-subtilis"
            )
        );
    }

    #[test]
    fn build_taxonomy_database_merged_with_target_uses_target_taxid_as_numeric_leaf(
    ) {
        let taxdump = Taxdump::new();

        taxdump
            .build(&[600], None, superkingdom_as_domain(), Some(true))
            .unwrap();

        let unit = taxdump.unit(600);
        assert_eq!(unit.rank, "s");
        assert_eq!(unit.accessions[0].accession, "ACC600.1");
        // BUG-R31: the numeric leaf must carry the merge target taxid so it
        // agrees with the text leaf. The unit keeps the old taxid, which is the
        // one BLAST hits report and join on.
        assert_eq!(unit.taxid, 600);
        assert_eq!(
            lineages(&unit),
            (
                "d__2;p__1239;g__1386;s__1423",
                "d__bacteria;p__bacillota;g__bacillus;s__bacillus-subtilis"
            )
        );
        assert!(taxdump.non_mapped().is_empty());
    }

    #[test]
    fn build_taxonomy_database_missing_or_empty_name_falls_back_to_taxid() {
        let taxdump = Taxdump::new();

        taxdump
            .build(&[900, 901], None, superkingdom_as_domain(), Some(true))
            .unwrap();

        assert_eq!(
            taxdump.unit(900).text_lineage,
            "d__bacteria;p__bacillota;g__bacillus;s__taxid-900"
        );
        assert_eq!(
            taxdump.unit(901).text_lineage,
            "d__bacteria;p__bacillota;g__bacillus;s__taxid-901"
        );
    }

    #[test]
    fn build_taxonomy_database_double_underscore_in_name_is_slugified() {
        let taxdump = Taxdump::new();

        taxdump
            .build(&[902], None, superkingdom_as_domain(), Some(true))
            .unwrap();

        assert_eq!(
            taxdump.unit(902).text_lineage,
            "d__bacteria;p__bacillota;g__bacillus;s__odd-name"
        );
    }

    // BUG-R16: every ancestor filtered out leaves only the leaf, with no
    // leading `;`.
    #[test]
    fn build_taxonomy_database_all_ancestors_filtered_has_only_leaf() {
        let taxdump = Taxdump::new();

        taxdump
            .build(&[1386], Some(vec![1, 131567, 2, 1783272, 1239]), None, None)
            .unwrap();

        assert_eq!(lineages(&taxdump.unit(1386)), ("g__1386", "g__bacillus"));
    }

    // BUG-R16: an empty lineage yields only the leaf, which parses downstream.
    #[test]
    fn build_taxonomy_database_empty_lineage_has_only_leaf() {
        let taxdump = Taxdump::new();

        taxdump.build(&[77], None, None, None).unwrap();

        let unit = taxdump.unit(77);
        assert_eq!(lineages(&unit), ("s__77", "s__orphan-species"));
        let mut hit = row("ACC77.1", 77, 100.0, 900, &unit.numeric_lineage);
        assert!(matches!(
            hit.parse_taxonomy().unwrap().taxonomy,
            Taxonomy::Parsed(_)
        ));
    }

    #[test]
    #[should_panic(expected = "PosOverflow")]
    fn build_taxonomy_database_tax_id_above_i32_max_panics() {
        let taxdump = Taxdump::new();
        let append = |name: &str, line: &str| {
            let path = taxdump.path(name);
            let content = read_to_string(&path).unwrap() + line;
            write(&path, content).unwrap();
        };
        append("nodes.dmp", "3000000000\t|\t1386\t|\tspecies\t|\n");
        append("taxidlineage.dmp", "3000000000\t|\t1 131567 2 \t|\n");

        let _ = taxdump.build(&[1423], None, None, None);
    }
}
