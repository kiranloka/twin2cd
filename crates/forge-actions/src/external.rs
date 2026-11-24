//! JSON-over-stdin contract for external executable actions.

use crate::{ActionError, ActionOutput};
use forge_plugin_sdk::ActionManifest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};
use tokio::{io::AsyncWriteExt, process::Command};

#[derive(Debug, Clone, Serialize)]
struct ExternalActionRequest<'a> {
    protocol_version: u32,
    action: &'a ActionManifest,
    inputs: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalActionResult {
    #[serde(default)]
    pub outputs: BTreeMap<String, Value>,
    #[serde(default)]
    pub summary: String,
}

#[derive(Debug, Default)]
pub struct ExternalActionRunner;

impl ExternalActionRunner {
    /// Runs an action executable with a minimal, explicit environment. The
    /// executable reads one JSON request from stdin and writes one JSON result
    /// to stdout; diagnostics belong on stderr.
    pub async fn run(
        executable: impl AsRef<Path>,
        working_directory: impl AsRef<Path>,
        manifest: &ActionManifest,
        inputs: BTreeMap<String, Value>,
        environment: &BTreeMap<String, String>,
    ) -> Result<ExternalActionResult, ActionError> {
        manifest.validate().map_err(ActionError::InvalidInput)?;
        let mut resolved_inputs = inputs;
        for (name, spec) in &manifest.inputs {
            if !resolved_inputs.contains_key(name) {
                if let Some(default) = &spec.default {
                    resolved_inputs.insert(name.clone(), default.clone());
                } else if spec.required {
                    return Err(ActionError::InvalidInput(format!(
                        "required action input '{name}' is missing"
                    )));
                }
            }
        }
        if let Some(unknown) = resolved_inputs
            .keys()
            .find(|name| !manifest.inputs.contains_key(*name))
        {
            return Err(ActionError::InvalidInput(format!(
                "action input '{unknown}' is not declared in the manifest"
            )));
        }

        let request = ExternalActionRequest {
            protocol_version: 1,
            action: manifest,
            inputs: resolved_inputs.clone(),
        };
        let encoded = serde_json::to_vec(&request)?;
        let executable = executable.as_ref();
        let mut child = Command::new(executable)
            .current_dir(working_directory)
            .env_clear()
            .envs(environment)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|source| ActionError::Start {
                program: executable.display().to_string(),
                source,
            })?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&encoded).await?;
        }
        let output = child.wait_with_output().await?;
        if !output.status.success() {
            let mut stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            for (name, input) in &resolved_inputs {
                if manifest.inputs.get(name).is_some_and(|spec| spec.sensitive) {
                    let secret = input
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| input.to_string());
                    if !secret.is_empty() {
                        stderr = stderr.replace(&secret, "[REDACTED]");
                    }
                }
            }
            return Err(ActionError::Failed {
                program: executable.display().to_string(),
                status: output.status.to_string(),
                stderr,
            });
        }
        let result: ExternalActionResult = serde_json::from_slice(&output.stdout)?;
        if let Some(unknown) = result
            .outputs
            .keys()
            .find(|name| !manifest.outputs.contains_key(*name))
        {
            return Err(ActionError::InvalidInput(format!(
                "external action returned undeclared output '{unknown}'"
            )));
        }
        Ok(result)
    }
}

impl From<ExternalActionResult> for ActionOutput {
    fn from(result: ExternalActionResult) -> Self {
        Self {
            stdout: serde_json::to_string(&result.outputs).unwrap_or_default(),
            stderr: result.summary,
        }
    }
}
