use crate::{
    domain::dtos::{
        blutils_output::BlutilsOutput, consensus_result::QueryWithConsensus,
    },
    use_cases::shared::write_or_append_to_file,
};

use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use std::{fs::remove_file, path::PathBuf};
use tracing::warn;

pub fn parse_consensus_as_tabular(
    mut blutils_result: PathBuf,
    mut output_file: PathBuf,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Validate input files
    // ? -----------------------------------------------------------------------

    blutils_result.set_extension("json");

    if !blutils_result.exists() {
        return use_case_err(format!(
            "The file `{}` does not exist.",
            blutils_result.to_str().unwrap()
        ))
        .as_error();
    }

    output_file.set_extension("tsv");

    if output_file.exists() {
        warn!("Output file already exists. Removing it.");
        remove_file(output_file.to_owned()).unwrap();
    }

    // ? -----------------------------------------------------------------------
    // ? Load content from Blutils output
    // ? -----------------------------------------------------------------------

    let content = match serde_json::from_str::<BlutilsOutput>(
        &std::fs::read_to_string(blutils_result).unwrap(),
    ) {
        Ok(res) => res,
        Err(err) => return use_case_err(format!("{err}")).as_error(),
    };

    // ? -----------------------------------------------------------------------
    // ? Write the tabular output
    // ? -----------------------------------------------------------------------

    let (file_writer, file) = write_or_append_to_file(&output_file);

    let columns = vec![
        "query",
        "rank",
        "identifier",
        "perc-identity",
        "bit-score",
        "taxonomy",
        "mutated",
        "single-match",
        "type",
        "occurrences",
        "accessions",
    ];

    file_writer(
        format!("{}\n", columns.join("\t")),
        file.try_clone()
            .expect("Unexpected error detected on write tabular output"),
    )?;

    let null = "null";

    for result in content.results {
        let bean = match result.to_owned() {
            QueryWithConsensus { query, taxon } => match taxon {
                Some(res) => res,
                None => {
                    if let Err(err) = file_writer(
                        format!("{}\tnull\n", query),
                        file.try_clone().expect(
                            "Unexpected error detected on write tabular output",
                        ),
                    ) {
                        panic!("Unexpected error detected on write sequences database: {err}");
                    };

                    continue;
                }
            },
        };

        // Write the first row
        let main_row_content = format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{null}\t{null}\n",
            result.query,
            "consensus",
            bean.reached_rank.as_full_rank_string(),
            bean.identifier,
            bean.perc_identity,
            bean.bit_score,
            bean.taxonomy.unwrap_or(null.to_string()),
            bean.mutated,
            bean.single_match,
        );

        if let Err(err) = file_writer(
            main_row_content,
            file.try_clone()
                .expect("Unexpected error detected on write tabular output"),
        ) {
            panic!(
                "Unexpected error detected on write sequences database: {err}"
            );
        };

        for consensus in bean.consensus_beans.unwrap_or_default() {
            let consensus_row_content = format!(
                "{}\t{}\t{}\t{}\t{null}\t{}\t{}\t{null}\t{null}\t{}\t{}\n",
                result.query,
                "blast-match",
                consensus.rank.as_full_rank_string(),
                consensus.identifier,
                bean.bit_score,
                consensus.taxonomy.unwrap_or(null.to_string()),
                consensus.occurrences,
                consensus.accessions.join(", "),
            );

            if let Err(err) = file_writer(
                consensus_row_content,
                file.try_clone().expect(
                    "Unexpected error detected on write tabular output",
                ),
            ) {
                panic!(
                    "Unexpected error detected on write sequences database: {err}"
                );
            };
        }
    }

    Ok(())
}
