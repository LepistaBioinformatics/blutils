use super::{shared::write_or_stdout, OutputFormat};
use crate::{
    domain::dtos::{
        consensus_result::QueryWithConsensus,
        file_or_stdin::{FileOrStdin, Source},
    },
    use_cases::shared::write_or_append_to_file,
};

use mycelium_base::utils::errors::{execution_err, use_case_err, MappedErrors};
use std::{fs::remove_file, path::PathBuf};
use tracing::warn;
use uuid::Uuid;

pub fn parse_consensus_as_tabular(
    blutils_result: FileOrStdin,
    output_file: Option<PathBuf>,
    result_format: OutputFormat,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Validate input files
    // ? -----------------------------------------------------------------------

    // Check the path that is actually read, whatever its extension
    // (BUG-R11).
    if let Source::Arg(ref file) = blutils_result.source {
        if !PathBuf::from(file).exists() {
            return use_case_err(format!("The file `{file}` does not exist."))
                .as_error();
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Load content from Blutils output
    // ? -----------------------------------------------------------------------

    let content = match result_format {
        OutputFormat::Json => blutils_result
            .json_content()
            .map_err(|e| use_case_err(format!("{e}")))?,
        OutputFormat::Jsonl => blutils_result
            .json_line_content()
            .map_err(|e| use_case_err(format!("{e}")))?,
        OutputFormat::Yaml => blutils_result
            .yaml_content()
            .map_err(|e| use_case_err(format!("{e}")))?,
    };

    // ? -----------------------------------------------------------------------
    // ? Write the tabular output
    // ? -----------------------------------------------------------------------

    let (file_writer, file) = {
        if let Some(mut out_file) = output_file.to_owned() {
            out_file.set_extension("tsv");

            // BUG-R43: file-system errors return `Err` instead of panicking.
            if out_file.exists() {
                warn!("Output file already exists. Removing it.");
                remove_file(&out_file).map_err(|err| {
                    execution_err(format!(
                        "Could not overwrite the output file {out_file:?}: {err}"
                    ))
                })?;
            }

            write_or_append_to_file(&out_file)?
        } else {
            write_or_append_to_file(&PathBuf::from("/dev/null"))?
        }
    };

    write_or_stdout(
        format!(
            "{}",
            vec![
                "run-id",
                "query",
                "type",
                "rank",
                "identifier",
                "perc-identity",
                "bit-score",
                "taxonomy",
                "mutated",
                "single-match",
                "occurrences",
                "accessions",
            ]
            .join("\t")
        ),
        file_writer,
        &file,
        output_file.is_none(),
    )?;

    let null = "null";

    let run_id = match content.config {
        Some(c) => c.run_id,
        None => Uuid::new_v4(),
    };

    for result in content.results {
        let bean = match result.to_owned() {
            QueryWithConsensus {
                query,
                taxon,
                run_id: _,
            } => match taxon {
                Some(res) => res,
                None => {
                    // Keep the 12-column layout of the header (BUG-R10).
                    write_or_stdout(
                        format!(
                            "{}\t{}{}",
                            result.run_id.as_ref().unwrap_or(&run_id),
                            query,
                            "\tnull".repeat(10)
                        ),
                        file_writer,
                        &file,
                        output_file.is_none(),
                    )?;

                    continue;
                }
            },
        };

        // Write the first row
        let main_row_content = format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{null}\t{null}",
            result.run_id.as_ref().unwrap_or(&run_id).to_string(),
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

        write_or_stdout(
            main_row_content.to_owned(),
            file_writer,
            &file,
            output_file.is_none(),
        )?;

        for consensus in bean.consensus_beans.unwrap_or_default() {
            let consensus_row_content = format!(
                "{}\t{}\t{}\t{}\t{}\t{null}\t{}\t{}\t{null}\t{null}\t{}\t{}",
                result.run_id.as_ref().unwrap_or(&run_id).to_string(),
                result.query,
                "blast-match",
                consensus.rank.as_full_rank_string(),
                consensus.identifier,
                bean.bit_score,
                consensus.taxonomy.unwrap_or(null.to_string()),
                consensus.occurrences,
                consensus.accessions.join(", "),
            );

            write_or_stdout(
                consensus_row_content,
                file_writer,
                &file,
                output_file.is_none(),
            )?;
        }
    }

    Ok(())
}
