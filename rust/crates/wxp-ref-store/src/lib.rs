use std::collections::HashMap;

#[derive(Default)]
pub struct RefStore {
    current_version: u64,
    refs: HashMap<(u64, u32), String>,
}

impl RefStore {
    pub fn replace_snapshot<S>(&mut self, _page_path: &str, version: u64, refs: Vec<(u32, S)>)
    where
        S: Into<String>,
    {
        self.current_version = version;
        self.refs
            .retain(|(existing_version, _), _| *existing_version == version);
        for (ref_id, selector) in refs {
            self.refs.insert((version, ref_id), selector.into());
        }
    }

    pub fn resolve(&self, ref_id: u32, version: u64) -> Option<String> {
        if version != self.current_version {
            return None;
        }
        self.refs.get(&(version, ref_id)).cloned()
    }
}
