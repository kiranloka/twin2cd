use forge_common::ExecutionId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub execution_id: ExecutionId,
    /// Shell command to run (`sh -c <command>` for native, `["sh","-c",command]` in containers).
    pub command: String,
    pub working_directory: String,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    /// Container image when the target executor is `container`.
    #[serde(default)]
    pub image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub exit_code: i32,
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
}
