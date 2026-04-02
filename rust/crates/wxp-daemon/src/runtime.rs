use std::{
    collections::HashMap,
    fmt,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use wxp_automator_client::AutomatorConnection;
use wxp_proxy::{LocalProxy, MockRegistry};
use wxp_ref_store::RefStore;
use wxp_snapshot::ElementNode;
use wxp_store::StoreWriter;

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SnapshotRefEntry {
    pub selector: String,
    pub text: Option<String>,
    pub placeholder: Option<String>,
}

pub struct DaemonRuntime {
    pub saved: DaemonState,
    pub connection: Option<AutomatorConnection>,
    pub ref_store: RefStore,
    pub snapshot_version: u64,
    pub snapshot_refs: HashMap<u32, SnapshotRefEntry>,
    pub request_store: StoreWriter,
    pub mock_registry: MockRegistry,
    pub proxy: LocalProxy,
}

impl DaemonRuntime {
    pub fn new(saved: DaemonState, db_file: &Path) -> io::Result<Self> {
        let request_store = StoreWriter::open(db_file).map_err(io::Error::other)?;
        Ok(Self::from_request_store(saved, request_store))
    }

    pub fn set_ws_endpoint_for_test(&mut self, endpoint: String) {
        self.saved.ws_endpoint = Some(endpoint);
    }

    pub fn replace_snapshot(&mut self, page_path: &str, tree: &[ElementNode]) -> u64 {
        self.snapshot_version += 1;

        let mut refs = Vec::new();
        let mut snapshot_refs = HashMap::new();
        collect_snapshot_refs(tree, &mut refs, &mut snapshot_refs);

        self.ref_store
            .replace_snapshot(page_path, self.snapshot_version, refs);
        self.snapshot_refs = snapshot_refs;
        self.snapshot_version
    }

    pub fn has_ref(&self, ref_id: u32) -> bool {
        self.ref_store
            .resolve(ref_id, self.snapshot_version)
            .is_some()
    }

    pub fn resolve_ref_selector(&self, ref_id: u32) -> Option<String> {
        self.ref_store.resolve(ref_id, self.snapshot_version)
    }

    pub fn read_ref_text(&self, ref_id: u32) -> Option<String> {
        self.snapshot_refs
            .get(&ref_id)
            .map(|entry| entry.text.clone().or_else(|| entry.placeholder.clone()))
            .flatten()
    }

    pub fn invalidate_refs(&mut self) {
        self.snapshot_refs.clear();
        self.ref_store = RefStore::default();
    }

    fn from_request_store(saved: DaemonState, request_store: StoreWriter) -> Self {
        Self {
            saved,
            connection: None,
            ref_store: RefStore::default(),
            snapshot_version: 0,
            snapshot_refs: HashMap::new(),
            request_store,
            mock_registry: MockRegistry::default(),
            proxy: LocalProxy::default(),
        }
    }
}

impl fmt::Debug for DaemonRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DaemonRuntime")
            .field("saved", &self.saved)
            .field("has_connection", &self.connection.is_some())
            .field("snapshot_version", &self.snapshot_version)
            .field("snapshot_ref_count", &self.snapshot_refs.len())
            .field("proxy_running", &self.proxy.is_running())
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub method: String,
    pub daemon_kind: &'static str,
}

fn collect_snapshot_refs(
    nodes: &[ElementNode],
    refs: &mut Vec<(u32, String)>,
    snapshot_refs: &mut HashMap<u32, SnapshotRefEntry>,
) {
    for node in nodes {
        if let Some(ref_id) = node.ref_id {
            refs.push((ref_id, node.selector.clone()));
            snapshot_refs.insert(
                ref_id,
                SnapshotRefEntry {
                    selector: node.selector.clone(),
                    text: node.text.clone(),
                    placeholder: node.placeholder.clone(),
                },
            );
        }
        collect_snapshot_refs(&node.children, refs, snapshot_refs);
    }
}
