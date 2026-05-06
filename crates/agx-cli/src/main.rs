mod agents;
mod cli;
mod commands;
mod config;
mod context;
mod doctor;
mod errors;
mod exec;
mod inspection;
mod lock;
mod output;
mod package_manager;
mod self_upgrade;
mod state;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::Cli;
use crate::commands::run_command;
use crate::context::CliContext;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let context = match CliContext::try_from(&cli) {
        Ok(context) => context,
        Err(error) => {
            eprintln!("{}", error.message);
            return ExitCode::from(error.exit_code());
        }
    };

    let result = run_command(&cli.command, &context);
    let exit_code = result.exit_code();

    if let Err(error) = output::emit_result(&result, &context) {
        eprintln!("failed to emit command result: {error}");
        return ExitCode::from(1);
    }

    ExitCode::from(exit_code)
}
