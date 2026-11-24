//! Secret storage and injection for workflow runs.
//!
//! Sources (merged, later wins):
//! 1. Process environment: `FORGE_SECRET_<NAME>` → secret `<NAME>`
//! 2. Values set at runtime via [`SecretsManager::set`] (API)

use std::collections::BTreeMap;
use std::sync::RwLock;

const ENV_PREFIX: &str = "FORGE_SECRET_";

#[derive(Debug, Default)]
pub struct SecretsManager {
    secrets: RwLock<BTreeMap<String, String>>,
}

impl SecretsManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load secrets from `FORGE_SECRET_*` process environment variables.
    pub fn from_env() -> Self {
        let mut secrets = BTreeMap::new();
        for (key, value) in std::env::vars() {
            if let Some(name) = key.strip_prefix(ENV_PREFIX) {
                if !name.is_empty() {
                    secrets.insert(name.to_string(), value);
                }
            }
        }
        Self {
            secrets: RwLock::new(secrets),
        }
    }

    pub fn set(&self, name: impl Into<String>, value: impl Into<String>) {
        self.secrets
            .write()
            .expect("secrets lock")
            .insert(name.into(), value.into());
    }

    pub fn get(&self, name: &str) -> Option<String> {
        self.secrets.read().expect("secrets lock").get(name).cloned()
    }

    pub fn remove(&self, name: &str) -> bool {
        self.secrets
            .write()
            .expect("secrets lock")
            .remove(name)
            .is_some()
    }

    /// Names of all known secrets (values never exposed by this method).
    pub fn names(&self) -> Vec<String> {
        self.secrets
            .read()
            .expect("secrets lock")
            .keys()
            .cloned()
            .collect()
    }

    /// Snapshot for expression evaluation.
    pub fn snapshot(&self) -> BTreeMap<String, String> {
        self.secrets.read().expect("secrets lock").clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_remove() {
        let m = SecretsManager::new();
        assert!(m.names().is_empty());
        m.set("TOKEN", "abc");
        assert_eq!(m.get("TOKEN").as_deref(), Some("abc"));
        assert_eq!(m.names(), vec!["TOKEN".to_string()]);
        assert!(m.remove("TOKEN"));
        assert_eq!(m.get("TOKEN"), None);
    }

    #[test]
    fn snapshot_is_isolated_copy() {
        let m = SecretsManager::new();
        m.set("A", "1");
        let snap = m.snapshot();
        m.set("B", "2");
        assert!(snap.contains_key("A"));
        assert!(!snap.contains_key("B"));
    }
}
