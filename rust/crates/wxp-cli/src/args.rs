use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DaemonKindArg {
    Rust,
    Ts,
}

#[derive(Debug, Parser)]
#[command(name = "wxpilot", version, about = "AI Agent 微信小程序自动化 CLI")]
pub struct Cli {
    #[arg(long = "daemon-kind", default_value = "rust")]
    pub daemon_kind: DaemonKindArg,

    #[arg(long, default_value_t = 10_000)]
    pub timeout: u64,

    #[arg(long, default_value_t = false)]
    pub verbose: bool,

    #[arg(long, default_value_t = false)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Activate(ActivateArgs),
    Start(StartArgs),
    Connect(ConnectArgs),
    Disconnect,
    Status,
    Goto(RouteArgs),
    Back,
    Reload,
    View(ViewArgs),
    Find(FindArgs),
    Wait(WaitArgs),
    Tap(RefArgs),
    Type(TypeArgs),
    Scroll(ScrollArgs),
    Read(RefArgs),
    Assert(AssertArgs),
    State(StateArgs),
    Shot(ShotArgs),
    Run(RunArgs),
    Wx(WxArgs),
    #[command(subcommand)]
    Net(NetCommands),
    #[command(subcommand)]
    Daemon(DaemonCommands),
}

#[derive(Debug, Args)]
pub struct ActivateArgs {
    pub key: String,
}

#[derive(Debug, Args)]
pub struct StartArgs {
    pub project_path: Option<String>,

    #[arg(long = "cli-path")]
    pub cli_path: Option<String>,

    #[arg(long)]
    pub appid: Option<String>,

    #[arg(long = "auto-port", default_value_t = 9420)]
    pub auto_port: u16,

    #[arg(long, default_value_t = false)]
    pub proxy: bool,

    #[arg(long, default_value_t = false)]
    pub https: bool,
}

#[derive(Debug, Args)]
pub struct ConnectArgs {
    #[arg(long = "ws")]
    pub ws_endpoint: Option<String>,
}

#[derive(Debug, Args)]
pub struct RouteArgs {
    pub route: String,
}

#[derive(Debug, Args)]
pub struct ViewArgs {
    #[arg(long, default_value_t = false)]
    pub full: bool,

    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    pub compact: Option<bool>,

    #[arg(long)]
    pub depth: Option<u32>,

    #[arg(long)]
    pub limit: Option<u32>,
}

#[derive(Debug, Args)]
pub struct FindArgs {
    pub query: String,

    #[arg(long, default_value_t = false)]
    pub all: bool,

    #[arg(long, default_value_t = 10)]
    pub limit: usize,
}

#[derive(Debug, Args)]
pub struct WaitArgs {
    pub reference: String,

    #[arg(long, default_value_t = 5_000)]
    pub timeout: u64,
}

#[derive(Debug, Args)]
pub struct RefArgs {
    pub reference: String,
}

#[derive(Debug, Args)]
pub struct TypeArgs {
    pub reference: String,
    pub text: String,
}

#[derive(Debug, Args)]
pub struct ScrollArgs {
    pub reference: String,
    pub direction: String,
}

#[derive(Debug, Args)]
pub struct AssertArgs {
    pub reference: String,
    pub expected: String,
}

#[derive(Debug, Args)]
pub struct StateArgs {
    pub path: Option<String>,

    #[arg(long, default_value_t = false)]
    pub full: bool,
}

#[derive(Debug, Args)]
pub struct ShotArgs {
    pub path: Option<String>,

    #[arg(long, default_value_t = false)]
    pub base64: bool,
}

#[derive(Debug, Args)]
pub struct RunArgs {
    pub js: String,
}

#[derive(Debug, Args)]
pub struct WxArgs {
    pub method: String,

    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}

#[derive(Debug, Subcommand)]
pub enum NetCommands {
    Start(NetStartArgs),
    Stop,
    Log(NetLogArgs),
    Mock(NetMockArgs),
    Unmock(NetUnmockArgs),
    Clear,
    InstallCa,
}

#[derive(Debug, Args)]
pub struct NetStartArgs {
    #[arg(long, default_value_t = 8899)]
    pub port: u16,

    #[arg(long, default_value_t = false)]
    pub https: bool,
}

#[derive(Debug, Args)]
pub struct NetLogArgs {
    #[arg(long)]
    pub filter: Option<String>,

    #[arg(long, default_value_t = 20)]
    pub limit: usize,

    #[arg(long = "with-body", default_value_t = false)]
    pub with_body: bool,
}

#[derive(Debug, Args)]
pub struct NetMockArgs {
    pub url: String,
    pub json_file: String,

    #[arg(long, default_value = "GET")]
    pub method: String,
}

#[derive(Debug, Args)]
pub struct NetUnmockArgs {
    pub url: String,
}

#[derive(Debug, Subcommand)]
pub enum DaemonCommands {
    Stop,
    Restart,
}
