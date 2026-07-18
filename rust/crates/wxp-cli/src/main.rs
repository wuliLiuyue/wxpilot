// SPDX-License-Identifier: AGPL-3.0-or-later
// wxpilot - 面向 AI Agent 的微信小程序自动化 CLI
// Copyright (C) 2026  wuliLiuyue
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use clap::Parser;
use wxp_cli::args::Cli;
use wxp_cli::command::CliCommand;
use wxp_cli::error::CliError;
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
