use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    UnsupportedDaemonKind,
    DaemonUnavailable(String),
    LicenseRequired,
    LicenseInvalid(String),
    ActivationFailed(String),
    Message(String),
    RefExpired(u64),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDaemonKind => f.write_str("ts daemon is no longer supported"),
            Self::DaemonUnavailable(message) => write!(f, "daemon_unavailable: {message}"),
            Self::LicenseRequired => {
                f.write_str("当前发布版需要先激活：请执行 `wxpilot activate <key>`")
            }
            Self::LicenseInvalid(message) => write!(f, "本地激活凭证无效：{message}"),
            Self::ActivationFailed(message) => write!(f, "激活失败：{message}"),
            Self::Message(message) => f.write_str(message),
            Self::RefExpired(reference) => write!(f, "ref_expired: %{reference}"),
        }
    }
}

impl std::error::Error for CliError {}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::UnsupportedDaemonKind => 2,
            Self::DaemonUnavailable(_)
            | Self::LicenseRequired
            | Self::LicenseInvalid(_)
            | Self::ActivationFailed(_)
            | Self::Message(_)
            | Self::RefExpired(_) => 1,
        }
    }
}
