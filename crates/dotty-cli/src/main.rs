mod cli;

use crate::cli::Cli;
use clap::Parser;

fn main() {
    if Cli::parse().run().is_err() {
        std::process::exit(1);
    }
}
