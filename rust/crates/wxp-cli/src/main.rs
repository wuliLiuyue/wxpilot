use clap::Parser;
use wxp_cli::args::Cli;
use wxp_cli::command::CliCommand;
use wxp_cli::error::CliError;
use wxp_cli::output::{render_error, render_ok, render_ref_expired, OutputMode};
use wxp_cli::runner::execute;

fn main() {
    let cli = Cli::parse();
    let mode = if cli.json {
        OutputMode::Json
    } else {
        OutputMode::Human
    };

    match CliCommand::try_from(cli).and_then(execute) {
        Ok(value) => println!("{}", render_ok(&value, mode)),
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
