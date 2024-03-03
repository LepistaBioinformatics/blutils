mod commands;

pub(crate) use commands::{
    Arguments, BuildConsensusArguments, Commands,
    RunBlastAndBuildConsensusArguments,
};

use blul_core::{
    domain::dtos::{
        blast_builder::BlastBuilder, parallel_blast_output::ParallelBlastOutput,
    },
    use_cases::{
        build_consensus_identities, check_host_requirements,
        parse_consensus_as_tabular, run_blast_and_build_consensus,
        write_blutils_output,
    },
};
use blul_proc::execute_blast::ExecuteBlastnProcRepository;
use std::path::{Path, PathBuf};

use self::commands::BuildTabularArguments;

pub(crate) fn run_blast_and_build_consensus_cmd(
    args: RunBlastAndBuildConsensusArguments,
) {
    // Execute system checks before running the blast
    if let Err(err) = check_host_requirements(Some("debug")) {
        panic!("{err}");
    }

    let repo = ExecuteBlastnProcRepository {};

    // Create configuration DTO
    let mut config = BlastBuilder::default(&args.database, args.taxon);

    if args.max_target_seqs.is_some() {
        config = config.with_max_target_seqs(args.max_target_seqs.unwrap());
    }

    if args.perc_identity.is_some() {
        config = config.with_perc_identity(args.perc_identity.unwrap());
    }

    if args.query_cov.is_some() {
        config = config.with_query_cov(args.query_cov.unwrap());
    }

    if args.strand.is_some() {
        config = config.with_strand(args.strand.unwrap());
    }

    if args.e_value.is_some() {
        config = config.with_e_value(args.e_value.unwrap());
    }

    if args.word_size.is_some() {
        config = config.with_word_size(args.word_size.unwrap());
    }

    // Set the default number of threads
    let threads = match args.threads {
        Some(n) => n,
        None => 1,
    };

    if let Err(err) = run_blast_and_build_consensus(
        &args.query,
        &args.tax_file,
        &args.out_dir,
        config,
        &repo,
        &args.force_overwrite,
        threads,
        args.strategy,
        Some(args.use_taxid),
    ) {
        panic!("{err}")
    };
}

pub(crate) fn build_consensus_cmd(args: BuildConsensusArguments) {
    let blast_output = match build_consensus_identities(
        ParallelBlastOutput {
            output_file: PathBuf::from(args.blast_out),
            headers: None,
        },
        Path::new(&args.tax_file),
        args.taxon,
        args.strategy,
        Some(args.use_taxid),
    ) {
        Ok(results) => results,
        Err(err) => panic!("{err}"),
    };

    write_blutils_output(
        blast_output.to_owned(),
        None,
        Path::new(&args.out_dir).to_path_buf(),
    );
}

pub(crate) fn build_tabular_cmd(args: BuildTabularArguments) {
    match parse_consensus_as_tabular(
        PathBuf::from(args.blu_result),
        PathBuf::from(args.tabular_output),
    ) {
        Ok(_) => (),
        Err(err) => panic!("{err}"),
    };
}
