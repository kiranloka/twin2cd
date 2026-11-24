//! OCI container action runner using the JSON-over-stdin SDK contract.

use crate::external::ExternalActionResult;
use crate::{check_output, ActionError};
use forge_plugin_sdk::ActionManifest;
use std::path::Path;
use tokio::{io::AsyncWriteExt, process::Command};

#[derive(Debug, Default)]
pub struct ContainerActionRunner;

impl ContainerActionRunner {
    /// Runs a digest-pinned action image. The JSON protocol request is passed
    /// on stdin so action inputs, including secrets, never appear in argv.
    pub async fn run(
        image: &str,
        working_directory: impl AsRef<Path>,
        request_json: &[u8],
        manifest: &ActionManifest,
    ) -> Result<ExternalActionResult, ActionError> {
        manifest.validate().map_err(ActionError::InvalidInput)?;
        let (repository, digest) = image.split_once("@sha256:").ok_or_else(|| {
            ActionError::InvalidInput(
                "container action image must be pinned with @sha256:<digest>".into(),
            )
        })?;
        if repository.is_empty()
            || digest.len() != 64
            || !digest
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            return Err(ActionError::InvalidInput(
                "container action image has an invalid sha256 digest".into(),
            ));
        }
        let working_directory = tokio::fs::canonicalize(working_directory).await?;
        if !working_directory.is_dir() {
            return Err(ActionError::InvalidInput(
                "container action working directory must be a directory".into(),
            ));
        }
        let mut child = Command::new("docker")
            .args(["run", "--rm", "--interactive", "--volume"])
            .arg(format!("{}:/workspace:rw", working_directory.display()))
            .args(["--workdir", "/workspace", image])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|source| ActionError::Start {
                program: "docker run container action".into(),
                source,
            })?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(request_json).await?;
        }
        let output = child.wait_with_output().await?;
        let result = check_output("docker run container action", output)?;
        let result: ExternalActionResult = serde_json::from_str(&result.stdout)?;
        if let Some(unknown) = result
            .outputs
            .keys()
            .find(|name| !manifest.outputs.contains_key(*name))
        {
            return Err(ActionError::InvalidInput(format!(
                "container action returned undeclared output '{unknown}'"
            )));
        }
        Ok(result)
    }
}
