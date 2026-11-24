pub mod archive;
pub mod container_action;
pub mod deployment;
pub mod docker;
pub mod external;
pub mod git;
pub mod http;
pub mod shell;

use std::process::Output;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ActionError {
    #[error("invalid action input: {0}")]
    InvalidInput(String),
    #[error("could not start {program}: {source}")]
    Start {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{program} exited with {status}: {stderr}")]
    Failed {
        program: String,
        status: String,
        stderr: String,
    },
    #[error("action I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("external action returned invalid JSON: {0}")]
    InvalidOutput(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionOutput {
    pub stdout: String,
    pub stderr: String,
}

pub(crate) fn check_output(program: &str, output: Output) -> Result<ActionOutput, ActionError> {
    if !output.status.success() {
        return Err(ActionError::Failed {
            program: program.to_owned(),
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(ActionOutput {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

pub(crate) fn validate_non_empty(name: &str, value: &str) -> Result<(), ActionError> {
    if value.trim().is_empty() {
        return Err(ActionError::InvalidInput(format!(
            "{name} must not be empty"
        )));
    }
    Ok(())
}
