extern crate blul;
mod cmds;

use clap::Parser;
use cmds::blast;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
enum Cli {
    Blast(blast::Arguments),
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    match args {
        Cli::Blast(sub_args) => {
            match sub_args.run_blast {
                blast::Commands::RunWithConsensus(args) => {
                    blast::run_blast_and_build_consensus_cmd(args)
                }
            };
        }
    };
}
