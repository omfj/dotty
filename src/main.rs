mod cli;

use crate::cli::Cli;
use clap::Parser;

fn main() {
    let cli = Cli::parse();

    if cli.run().is_err() {
        std::process::exit(1);
    }
}
