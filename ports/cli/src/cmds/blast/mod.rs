mod commands;

use self::commands::BuildTabularArguments;
pub(crate) use commands::{
    Arguments, BuildConsensusArguments, Commands,
    RunBlastAndBuildConsensusArguments,
};

use anyhow::{anyhow, bail, Result};
use blul_core::{
    domain::dtos::{
        blast_builder::BlastBuilder,
        parallel_blast_output::ParallelBlastOutput,
        taxon::{CustomTaxon, Taxon},
    },
    use_cases::{
        build_consensus_identities, check_host_requirements,
        parse_consensus_as_tabular, run_blast_and_build_consensus,
        write_blutils_output,
    },
};
use blul_proc::execute_blast::ExecuteBlastnProcRepository;
use std::path::{Path, PathBuf};

pub(crate) fn run_blast_and_build_consensus_cmd(
    args: RunBlastAndBuildConsensusArguments,
    threads: Option<usize>,
) -> Result<()> {
    // Execute system checks before running the blast
    check_host_requirements(Some("debug")).map_err(|err| anyhow!("{err}"))?;

    let repo = ExecuteBlastnProcRepository {};

    // Create configuration DTO
    let mut config =
        BlastBuilder::default(&args.database, args.taxon.to_owned());

    if let Some(value) = args.max_target_seqs {
        config = config.with_max_target_seqs(value);
    }

    if let Some(value) = args.perc_identity {
        config = config.with_perc_identity(value);
    }

    if let Some(value) = args.query_cov {
        config = config.with_query_cov(value);
    }

    if let Some(value) = args.strand {
        config = config.with_strand(value);
    }

    if let Some(value) = args.e_value {
        config = config.with_e_value(value);
    }

    if let Some(value) = args.word_size {
        config = config.with_word_size(value);
    }

    // Set the default number of threads
    let threads = match threads {
        Some(n) => n,
        None => 1,
    };

    let custom_taxon =
        load_custom_taxon(args.custom_taxon_cutoff_file, &args.taxon)?;

    run_blast_and_build_consensus(
        args.query,
        &args.tax_file,
        &args.blast_out_file,
        args.blutils_out_file,
        config,
        &repo,
        &args.force_overwrite,
        threads,
        args.strategy,
        Some(args.use_taxid),
        args.out_format,
        custom_taxon,
    )
    .map_err(|err| anyhow!("{err}"))?;

    Ok(())
}

/// Reads the custom cutoffs file, required when the custom taxon is selected.
fn load_custom_taxon(
    file: Option<PathBuf>,
    taxon: &Taxon,
) -> Result<Option<CustomTaxon>> {
    match file {
        Some(file) => CustomTaxon::from_file(file)
            .map(Some)
            .map_err(|err| anyhow!("{err}")),
        None => {
            if let Taxon::Custom = taxon {
                bail!(
                    "Custom taxon values are required when the custom taxon option is selected (use --custom-taxon-cutoff-file)."
                );
            }

            Ok(None)
        }
    }
}

pub(crate) fn build_consensus_cmd(args: BuildConsensusArguments) -> Result<()> {
    let custom_taxon =
        load_custom_taxon(args.custom_taxon_cutoff_file, &args.taxon)?;

    let blast_output = match build_consensus_identities(
        ParallelBlastOutput {
            output_file: PathBuf::from(args.blast_out),
            headers: None,
        },
        Path::new(&args.tax_file),
        args.taxon,
        args.strategy,
        Some(args.use_taxid),
        custom_taxon,
    ) {
        Ok(results) => results,
        Err(err) => bail!("{err}"),
    };

    write_blutils_output(
        blast_output.to_owned(),
        None,
        args.blutils_out_file,
        args.out_format,
    )
    .map_err(|err| anyhow!("{err}"))
}

pub(crate) fn build_tabular_cmd(args: BuildTabularArguments) -> Result<()> {
    parse_consensus_as_tabular(
        args.blu_result,
        match args.output_file {
            Some(file) => Some(PathBuf::from(file)),
            None => None,
        },
        args.input_format,
    )
    .map_err(|err| anyhow!("{err}"))
}
