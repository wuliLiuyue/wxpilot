use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    DaemonUnavailable(String),
    Message(String),
    RefExpired(u64),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DaemonUnavailable(message) => write!(f, "daemon_unavailable: {message}"),
            Self::Message(message) => f.write_str(message),
            Self::RefExpired(reference) => write!(f, "ref_expired: %{reference}"),
        }
    }
}

impl std::error::Error for CliError {}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::DaemonUnavailable(_) | Self::Message(_) | Self::RefExpired(_) => 1,
        }
    }
}
