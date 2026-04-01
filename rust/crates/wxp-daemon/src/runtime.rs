use std::{
    fmt,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    process,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use wxp_automator_client::AutomatorConnection;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct DaemonState {
    pub project_path: Option<String>,
    pub ws_endpoint: Option<String>,
    pub connected: bool,
}

#[derive(Debug, Clone)]
pub struct StateStore {
    path: PathBuf,
}

impl StateStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> io::Result<DaemonState> {
        if !self.path.exists() {
            return Ok(DaemonState::default());
        }

        let content = fs::read_to_string(&self.path)?;
        if content.trim().is_empty() {
            return Ok(DaemonState::default());
        }

        serde_json::from_str(&content)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    pub fn save(&self, state: &DaemonState) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(state).map_err(io::Error::other)?;
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let temp_path = self
            .path
            .with_extension(format!("tmp-{}-{nanos}", process::id()));

        let mut temp_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)?;
        temp_file.write_all(content.as_bytes())?;
        temp_file.sync_all()?;
        drop(temp_file);

        match fs::rename(&temp_path, &self.path) {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = fs::remove_file(&temp_path);
                Err(error)
            }
        }
    }
}

pub struct DaemonRuntime {
    pub saved: DaemonState,
    pub connection: Option<AutomatorConnection>,
}

impl DaemonRuntime {
    pub fn new(saved: DaemonState) -> Self {
        Self {
            saved,
            connection: None,
        }
    }

    pub fn set_ws_endpoint_for_test(&mut self, endpoint: String) {
        self.saved.ws_endpoint = Some(endpoint);
    }
}

impl fmt::Debug for DaemonRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DaemonRuntime")
            .field("saved", &self.saved)
            .field("has_connection", &self.connection.is_some())
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub method: String,
    pub daemon_kind: &'static str,
}
