use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDocument {
    pub version: String,
    pub name: String,
    #[serde(default)]
    pub triggers: Vec<Trigger>,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    #[serde(default)]
    pub variables: BTreeMap<String, String>,
    pub jobs: BTreeMap<String, JobSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Trigger {
    GitPush { branches: Vec<String> },
    Manual,
    Schedule { cron: String },
    Webhook,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    pub name: String,
    #[serde(default)]
    pub needs: Vec<String>,
    #[serde(default)]
    pub runs_on: RunsOn,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub timeout_minutes: Option<u64>,
    pub steps: Vec<StepSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunsOn {
    #[serde(default)]
    pub executor: ExecutorKind,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorKind {
    #[default]
    Native,
    Container,
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepSpec {
    pub name: String,
    #[serde(default)]
    pub run: Option<String>,
    #[serde(default)]
    pub uses: Option<String>,
    #[serde(default)]
    pub with: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub timeout_minutes: Option<u64>,
}
