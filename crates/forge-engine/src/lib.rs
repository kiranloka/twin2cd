//! In-process run lifecycle engine.
//!
//! Owns the full path from workflow trigger to terminal run state:
//! creates runs/jobs/steps in a [`RunStore`], gates jobs on `needs`,
//! executes steps with layered env/secrets, enforces timeouts, and
//! supports cooperative cancellation.

mod driver;
mod step;

/// Default job timeout when a job does not set `timeout_minutes`.
pub const DEFAULT_JOB_TIMEOUT_MINUTES: u64 = 60;

use forge_common::RunId;
use forge_core::{ForgeError, ForgeResult, JobRun, Run, StepRun};
use forge_executor::{ContainerExecutor, NativeExecutor};
use forge_secrets::SecretsManager;
use forge_storage::RunStore;
use forge_workflow::CompiledWorkflow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

/// Tuning knobs for [`RunEngine`].
#[derive(Debug, Clone)]
pub struct RunEngineConfig {
    /// Upper bound on concurrently running jobs (across all runs).
    pub max_concurrent_jobs: usize,
    /// Default job timeout when `timeout_minutes` is not set.
    pub default_job_timeout_minutes: u64,
    /// Directory where per-job workspaces are created.
    pub workspace_root: PathBuf,
}

impl Default for RunEngineConfig {
    fn default() -> Self {
        Self {
            max_concurrent_jobs: 4,
            default_job_timeout_minutes: DEFAULT_JOB_TIMEOUT_MINUTES,
            workspace_root: std::env::temp_dir().join("forge-workspace"),
        }
    }
}

/// Shared engine internals, cheap to clone into driver tasks.
#[derive(Clone)]
pub(crate) struct EngineShared {
    pub store: Arc<dyn RunStore>,
    pub secrets: Arc<SecretsManager>,
    pub native: Arc<NativeExecutor>,
    pub container: Option<Arc<ContainerExecutor>>,
    pub config: RunEngineConfig,
    pub job_semaphore: Arc<tokio::sync::Semaphore>,
    pub cancellations: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

/// The in-process execution engine shared by the API server.
pub struct RunEngine {
    shared: EngineShared,
}

impl RunEngine {
    pub fn new(
        store: Arc<dyn RunStore>,
        secrets: Arc<SecretsManager>,
        config: RunEngineConfig,
    ) -> Self {
        let container = match ContainerExecutor::connect() {
            Ok(c) => Some(Arc::new(c)),
            Err(e) => {
                tracing::warn!(error = %e, "docker unavailable — container jobs will fail");
                None
            }
        };
        let permits = config.max_concurrent_jobs.max(1);
        Self {
            shared: EngineShared {
                store,
                secrets,
                native: Arc::new(NativeExecutor::new()),
                container,
                config,
                job_semaphore: Arc::new(tokio::sync::Semaphore::new(permits)),
                cancellations: Arc::new(Mutex::new(HashMap::new())),
            },
        }
    }

    /// Trigger a compiled workflow. Returns the new run id immediately;
    /// execution continues on a background task.
    pub async fn trigger(&self, compiled: CompiledWorkflow, trigger: &str) -> ForgeResult<RunId> {
        let run_id = RunId::new(format!("run_{}", uuid::Uuid::new_v4().simple()));
        let mut run = Run::new(run_id.clone(), compiled.document.name.clone(), trigger);

        let jobs = build_job_runs(&run, &compiled);
        run.status = forge_core::RunStatus::InProgress;
        run.started_at_ms = Some(forge_core::now_ms());

        self.shared.store.create_run(run, jobs).await?;

        let token = CancellationToken::new();
        self.shared
            .cancellations
            .lock()
            .expect("engine cancel lock")
            .insert(run_id.to_string(), token.clone());

        let shared = self.shared.clone();
        let driving_run_id = run_id.clone();
        tokio::spawn(async move {
            driver::drive_run(shared, driving_run_id, compiled, token).await;
        });

        tracing::info!(%run_id, trigger, "run triggered");
        Ok(run_id)
    }

    /// Cancel a non-terminal run. Errors if the run already finished.
    pub async fn cancel(&self, run_id: &RunId) -> ForgeResult<()> {
        let run = self.shared.store.get_run(run_id).await?;
        if run.status.is_terminal() {
            return Err(ForgeError::Execution(format!(
                "run '{run_id}' already finished ({:?})",
                run.status
            )));
        }

        let token = self
            .shared
            .cancellations
            .lock()
            .expect("engine cancel lock")
            .get(run_id.as_str())
            .cloned();

        match token {
            Some(token) => {
                token.cancel();
                tracing::info!(%run_id, "cancel requested");
                Ok(())
            }
            None => Err(ForgeError::Execution(format!(
                "run '{run_id}' has no active driver"
            ))),
        }
    }

    pub fn secrets(&self) -> Arc<SecretsManager> {
        Arc::clone(&self.shared.secrets)
    }

    pub fn store(&self) -> Arc<dyn RunStore> {
        Arc::clone(&self.shared.store)
    }
}

fn build_job_runs(run: &Run, compiled: &CompiledWorkflow) -> Vec<JobRun> {
    compiled
        .document
        .jobs
        .iter()
        .map(|(key, spec)| {
            let job_id = forge_common::JobId::new(format!("{}::{}", run.id, key));
            let steps = spec
                .steps
                .iter()
                .enumerate()
                .map(|(i, s)| StepRun::new(i as u32, s.name.clone(), s.timeout_minutes))
                .collect();
            JobRun::new(
                job_id,
                run.id.clone(),
                key.clone(),
                spec.name.clone(),
                spec.needs.clone(),
                spec.timeout_minutes,
                spec.env.clone(),
                steps,
            )
        })
        .collect()
}
