use std::collections::HashMap;

#[derive(Default)]
pub struct MockRegistry {
    items: HashMap<String, String>,
}

impl MockRegistry {
    pub fn upsert(&mut self, method: &str, url: &str, body: &str) {
        self.items
            .insert(format!("{method}:{url}"), body.to_string());
    }

    pub fn remove(&mut self, url: &str) {
        self.items.retain(|key, _| !key.ends_with(url));
    }

    pub fn match_url(&self, method: &str, url: &str) -> Option<&String> {
        self.items.get(&format!("{method}:{url}"))
    }
}
