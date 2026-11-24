//! Runtime entities for workflow executions: runs, jobs, and steps.

use forge_common::{JobId, RunId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl RunStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            RunStatus::Completed | RunStatus::Failed | RunStatus::Cancelled
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Skipped,
    Cancelled,
}

impl JobStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            JobStatus::Completed | JobStatus::Failed | JobStatus::Skipped | JobStatus::Cancelled
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Cancelled,
    TimedOut,
}

impl StepStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            StepStatus::Completed
                | StepStatus::Failed
                | StepStatus::Skipped
                | StepStatus::Cancelled
                | StepStatus::TimedOut
        )
    }

    pub fn is_success(self) -> bool {
        matches!(self, StepStatus::Completed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: RunId,
    pub workflow_name: String,
    pub status: RunStatus,
    pub trigger: String,
    pub created_at_ms: u64,
    pub started_at_ms: Option<u64>,
    pub finished_at_ms: Option<u64>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
}

impl Run {
    pub fn new(id: RunId, workflow_name: impl Into<String>, trigger: impl Into<String>) -> Self {
        Self {
            id,
            workflow_name: workflow_name.into(),
            status: RunStatus::Pending,
            trigger: trigger.into(),
            created_at_ms: now_ms(),
            started_at_ms: None,
            finished_at_ms: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRun {
    pub id: JobId,
    pub run_id: RunId,
    /// Key of the job in the workflow document (`jobs.<key>`).
    pub job_key: String,
    pub name: String,
    pub status: JobStatus,
    #[serde(default)]
    pub needs: Vec<String>,
    pub timeout_minutes: Option<u64>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub steps: Vec<StepRun>,
    #[serde(default)]
    pub error: Option<String>,
    pub created_at_ms: u64,
    pub started_at_ms: Option<u64>,
    pub finished_at_ms: Option<u64>,
}

impl JobRun {
    pub fn new(
        id: JobId,
        run_id: RunId,
        job_key: impl Into<String>,
        name: impl Into<String>,
        needs: Vec<String>,
        timeout_minutes: Option<u64>,
        env: BTreeMap<String, String>,
        steps: Vec<StepRun>,
    ) -> Self {
        Self {
            id,
            run_id,
            job_key: job_key.into(),
            name: name.into(),
            status: JobStatus::Pending,
            needs,
            timeout_minutes,
            env,
            steps,
            error: None,
            created_at_ms: now_ms(),
            started_at_ms: None,
            finished_at_ms: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRun {
    pub index: u32,
    pub name: String,
    pub status: StepStatus,
    #[serde(default)]
    pub timeout_minutes: Option<u64>,
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
    pub created_at_ms: u64,
    pub started_at_ms: Option<u64>,
    pub finished_at_ms: Option<u64>,
}

impl StepRun {
    pub fn new(index: u32, name: impl Into<String>, timeout_minutes: Option<u64>) -> Self {
        Self {
            index,
            name: name.into(),
            status: StepStatus::Pending,
            timeout_minutes,
            exit_code: None,
            error: None,
            stdout: String::new(),
            stderr: String::new(),
            created_at_ms: now_ms(),
            started_at_ms: None,
            finished_at_ms: None,
        }
    }
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
