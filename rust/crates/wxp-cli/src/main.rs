use clap::Parser;
use wxp_cli::args::Cli;
use wxp_cli::command::CliCommand;
use wxp_cli::error::CliError;
use wxp_cli::license::ensure_license;
use wxp_cli::output::{render_command_ok, render_error, render_ref_expired, OutputMode};
use wxp_cli::runner::execute;

fn main() {
    let cli = Cli::parse();
    let mode = if cli.json {
        OutputMode::Json
    } else {
        OutputMode::Human
    };

    let command = match CliCommand::try_from(cli) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("{}", render_error(&error.to_string(), None, mode));
            std::process::exit(error.exit_code());
        }
    };

    if !command.is_license_exempt() {
        if let Err(error) = ensure_license() {
            eprintln!("{}", render_error(&error.to_string(), None, mode));
            std::process::exit(error.exit_code());
        }
    }

    match execute(command.clone()) {
        Ok(value) => println!("{}", render_command_ok(&command, &value, mode)),
        Err(CliError::RefExpired(reference)) => {
            eprintln!("{}", render_ref_expired(reference, mode));
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("{}", render_error(&error.to_string(), None, mode));
            std::process::exit(error.exit_code());
        }
    }
}
