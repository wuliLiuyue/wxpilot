use std::convert::TryFrom;

use crate::args::{Cli, Commands, DaemonCommands, DaemonKindArg, FindArgs, NetCommands, StartArgs};
use crate::error::CliError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonKindOption {
    Rust,
    Ts,
}

impl DaemonKindOption {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Ts => "ts",
        }
    }
}

impl From<DaemonKindArg> for DaemonKindOption {
    fn from(value: DaemonKindArg) -> Self {
        match value {
            DaemonKindArg::Rust => Self::Rust,
            DaemonKindArg::Ts => Self::Ts,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Start {
        project_path: Option<String>,
        cli_path: Option<String>,
        appid: Option<String>,
        auto_port: u16,
        proxy: bool,
        proxy_https: bool,
        timeout_ms: u64,
    },
    Connect {
        ws_endpoint: Option<String>,
        timeout_ms: u64,
    },
    Disconnect,
    Status,
    Goto {
        route: String,
        timeout_ms: u64,
    },
    Back,
    Reload,
    View {
        full: bool,
        compact: Option<bool>,
        depth: Option<u32>,
        limit: Option<u32>,
    },
    Find {
        query: String,
        all: bool,
        limit: usize,
    },
    Wait {
        reference: String,
        timeout_ms: u64,
    },
    Tap {
        reference: String,
    },
    Type {
        reference: String,
        text: String,
    },
    Scroll {
        reference: String,
        direction: String,
    },
    Read {
        reference: String,
    },
    Assert {
        reference: String,
        expected: String,
    },
    State {
        path: Option<String>,
        full: bool,
    },
    Shot {
        path: Option<String>,
        base64: bool,
    },
    Run {
        js: String,
    },
    Wx {
        method: String,
        args: Vec<String>,
    },
    NetStart {
        port: u16,
        https: bool,
        timeout_ms: u64,
    },
    NetStop {
        timeout_ms: u64,
    },
    NetLog {
        filter: Option<String>,
        limit: usize,
        with_body: bool,
        json: bool,
    },
    NetMock {
        url: String,
        json_file: String,
        method: String,
    },
    NetUnmock {
        url: String,
    },
    NetClear,
    NetInstallCa,
    DaemonStop,
    DaemonRestart,
}

impl TryFrom<Cli> for CliCommand {
    type Error = CliError;

    fn try_from(cli: Cli) -> Result<Self, Self::Error> {
        if matches!(cli.daemon_kind, DaemonKindArg::Ts) {
            return Err(CliError::UnsupportedDaemonKind);
        }

        let json = cli.json;
        let timeout_ms = cli.timeout;

        match cli.command {
            Commands::Start(StartArgs {
                project_path,
                cli_path,
                appid,
                auto_port,
                proxy,
                https,
            }) => Ok(Self::Start {
                project_path,
                cli_path,
                appid,
                auto_port,
                proxy: proxy || https,
                proxy_https: https,
                timeout_ms: 35_000,
            }),
            Commands::Connect(args) => Ok(Self::Connect {
                ws_endpoint: args.ws_endpoint,
                timeout_ms: 30_000,
            }),
            Commands::Disconnect => Ok(Self::Disconnect),
            Commands::Status => Ok(Self::Status),
            Commands::Goto(args) => Ok(Self::Goto {
                route: args.route,
                timeout_ms,
            }),
            Commands::Back => Ok(Self::Back),
            Commands::Reload => Ok(Self::Reload),
            Commands::View(args) => Ok(Self::View {
                full: args.full,
                compact: args.compact,
                depth: args.depth,
                limit: args.limit,
            }),
            Commands::Find(FindArgs { query, all, limit }) => Ok(Self::Find { query, all, limit }),
            Commands::Wait(args) => Ok(Self::Wait {
                reference: args.reference,
                timeout_ms: args.timeout,
            }),
            Commands::Tap(args) => Ok(Self::Tap {
                reference: args.reference,
            }),
            Commands::Type(args) => Ok(Self::Type {
                reference: args.reference,
                text: args.text,
            }),
            Commands::Scroll(args) => Ok(Self::Scroll {
                reference: args.reference,
                direction: args.direction,
            }),
            Commands::Read(args) => Ok(Self::Read {
                reference: args.reference,
            }),
            Commands::Assert(args) => Ok(Self::Assert {
                reference: args.reference,
                expected: args.expected,
            }),
            Commands::State(args) => Ok(Self::State {
                path: args.path,
                full: args.full,
            }),
            Commands::Shot(args) => Ok(Self::Shot {
                path: args.path,
                base64: args.base64,
            }),
            Commands::Run(args) => Ok(Self::Run { js: args.js }),
            Commands::Wx(args) => Ok(Self::Wx {
                method: args.method,
                args: args.args,
            }),
            Commands::Net(net) => match net {
                NetCommands::Start(args) => Ok(Self::NetStart {
                    port: args.port,
                    https: args.https,
                    timeout_ms: 30_000,
                }),
                NetCommands::Stop => Ok(Self::NetStop { timeout_ms: 15_000 }),
                NetCommands::Log(args) => Ok(Self::NetLog {
                    filter: args.filter,
                    limit: args.limit,
                    with_body: args.with_body,
                    json,
                }),
                NetCommands::Mock(args) => Ok(Self::NetMock {
                    url: args.url,
                    json_file: args.json_file,
                    method: args.method,
                }),
                NetCommands::Unmock(args) => Ok(Self::NetUnmock { url: args.url }),
                NetCommands::Clear => Ok(Self::NetClear),
                NetCommands::InstallCa => Ok(Self::NetInstallCa),
            },
            Commands::Daemon(daemon) => match daemon {
                DaemonCommands::Stop => Ok(Self::DaemonStop),
                DaemonCommands::Restart => Ok(Self::DaemonRestart),
            },
        }
    }
}
