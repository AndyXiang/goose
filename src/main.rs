use clap::Parser;
use goose::{cli::Cli, error::Result};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    dotenvy::dotenv().ok();

    match Cli::parse() {
        Cli::Db { action } => action.act(),
    }
}
