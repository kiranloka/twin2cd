use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub inputs: BTreeMap<String, ActionInput>,
    #[serde(default)]
    pub outputs: BTreeMap<String, ActionOutput>,
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionInput {
    pub description: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<Value>,
    #[serde(default)]
    pub sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionOutput {
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionReference {
    pub name: String,
    pub version: String,
}

impl ActionReference {
    /// Parse an immutable action reference such as `forge/aws-ecr-push@1.2.0`.
    pub fn parse(reference: &str) -> Result<Self, String> {
        let (name, version) = reference
            .rsplit_once('@')
            .ok_or_else(|| "action reference must include @version".to_owned())?;
        if name.trim().is_empty() || version.trim().is_empty() {
            return Err("action name and version must not be empty".into());
        }
        if !name.split('/').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        }) {
            return Err("action name contains unsupported characters".into());
        }
        if !version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
        {
            return Err("action version contains unsupported characters".into());
        }
        Ok(Self {
            name: name.to_owned(),
            version: version.to_owned(),
        })
    }
}

impl ActionManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("action name is required".into());
        }
        if self.version.trim().is_empty() {
            return Err("action version is required".into());
        }
        ActionReference::parse(&format!("{}@{}", self.name, self.version))?;
        for (name, input) in &self.inputs {
            if name.trim().is_empty() {
                return Err("input names must not be empty".into());
            }
            if input.required && input.default.is_some() {
                return Err(format!("required input '{name}' cannot define a default"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ActionRegistryError {
    #[error("invalid action manifest: {0}")]
    InvalidManifest(String),
    #[error("action version '{name}@{version}' is already registered")]
    VersionExists { name: String, version: String },
}

/// In-process registry; published versions are immutable once registered.
#[derive(Debug, Default)]
pub struct ActionRegistry {
    actions: BTreeMap<String, BTreeMap<String, ActionManifest>>,
}

impl ActionRegistry {
    pub fn register(&mut self, manifest: ActionManifest) -> Result<(), ActionRegistryError> {
        manifest
            .validate()
            .map_err(ActionRegistryError::InvalidManifest)?;
        let versions = self.actions.entry(manifest.name.clone()).or_default();
        if versions.contains_key(&manifest.version) {
            return Err(ActionRegistryError::VersionExists {
                name: manifest.name,
                version: manifest.version,
            });
        }
        versions.insert(manifest.version.clone(), manifest);
        Ok(())
    }

    pub fn resolve(&self, reference: &ActionReference) -> Option<&ActionManifest> {
        self.actions
            .get(&reference.name)
            .and_then(|versions| versions.get(&reference.version))
    }

    pub fn versions(&self, name: &str) -> impl Iterator<Item = &ActionManifest> {
        self.actions
            .get(name)
            .into_iter()
            .flat_map(|versions| versions.values())
    }
}
