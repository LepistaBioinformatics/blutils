extern crate blul;

use blul::{
    adapters::proc::execute_step::ExecuteStepProcRepository,
    domain::dtos::blast_builder::BlastBuilder,
    use_cases::run_blast_and_build_consensus,
};
use std::env::set_var;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Cli {
    query: String,
    subject: String,
    tax_file: String,
    out_dir: String,

    #[structopt(short = "f", long = "force-overwrite")]
    overwrite: bool,

    #[structopt(short = "t")]
    threads: Option<usize>,
}

fn main() {
    // Build logger
    set_var("RUST_LOG", "debug");
    env_logger::init();

    // Build cli arguments
    let args = Cli::from_args();

    // Initialize Execution repository
    let repo = ExecuteStepProcRepository {};

    // Create configuration DTO
    let config = BlastBuilder::create(&args.subject);

    // Set the default number of threads
    let threads = match args.threads {
        Some(n) => n,
        None => 1,
    };

    // Run Blast
    let blast_output = run_blast_and_build_consensus(
        &args.query,
        &args.tax_file,
        &args.out_dir,
        config,
        &repo,
        &args.overwrite,
        threads,
    );

    println!("{:?}", blast_output);
}
