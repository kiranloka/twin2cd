//! Run/job persistence for the milestone-2 in-process engine.
//!
//! [`RunStore`] is the trait the engine depends on; [`MemoryStore`] is the
//! in-memory implementation. A SQLite-backed implementation can implement the
//! same trait later without touching the engine.

use async_trait::async_trait;
use forge_common::{JobId, RunId};
use forge_core::{ForgeError, ForgeResult, JobRun, Run};
use std::collections::BTreeMap;
use std::sync::RwLock;

#[async_trait]
pub trait RunStore: Send + Sync {
    async fn create_run(&self, run: Run, jobs: Vec<JobRun>) -> ForgeResult<()>;
    async fn get_run(&self, run_id: &RunId) -> ForgeResult<Run>;
    async fn list_runs(&self) -> ForgeResult<Vec<Run>>;
    async fn update_run(&self, run: Run) -> ForgeResult<()>;

    async fn get_job(&self, job_id: &JobId) -> ForgeResult<JobRun>;
    async fn list_jobs_for_run(&self, run_id: &RunId) -> ForgeResult<Vec<JobRun>>;
    async fn update_job(&self, job: JobRun) -> ForgeResult<()>;
}

#[derive(Debug, Default)]
struct MemoryState {
    runs: BTreeMap<String, Run>,
    /// run_id → job_id → job (insertion order preserved by BTreeMap on job id;
    /// callers sort by created_at when order matters).
    jobs: BTreeMap<String, BTreeMap<String, JobRun>>,
}

/// Thread-safe in-memory [`RunStore`].
#[derive(Debug, Default)]
pub struct MemoryStore {
    state: RwLock<MemoryState>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

fn not_found(what: &str, id: &str) -> ForgeError {
    ForgeError::NotFound(format!("{what} '{id}' not found"))
}

#[async_trait]
impl RunStore for MemoryStore {
    async fn create_run(&self, run: Run, jobs: Vec<JobRun>) -> ForgeResult<()> {
        let mut state = self.state.write().expect("memory store lock");
        let run_id = run.id.to_string();
        let mut job_map = BTreeMap::new();
        for job in jobs {
            job_map.insert(job.id.to_string(), job);
        }
        if state.runs.contains_key(&run_id) {
            return Err(ForgeError::Storage(format!(
                "run '{run_id}' already exists"
            )));
        }
        state.runs.insert(run_id.clone(), run);
        state.jobs.insert(run_id, job_map);
        Ok(())
    }

    async fn get_run(&self, run_id: &RunId) -> ForgeResult<Run> {
        self.state
            .read()
            .expect("memory store lock")
            .runs
            .get(run_id.as_str())
            .cloned()
            .ok_or_else(|| not_found("run", run_id.as_str()))
    }

    async fn list_runs(&self) -> ForgeResult<Vec<Run>> {
        let state = self.state.read().expect("memory store lock");
        let mut runs: Vec<Run> = state.runs.values().cloned().collect();
        runs.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));
        Ok(runs)
    }

    async fn update_run(&self, run: Run) -> ForgeResult<()> {
        let mut state = self.state.write().expect("memory store lock");
        let run_id = run.id.to_string();
        if !state.runs.contains_key(&run_id) {
            return Err(not_found("run", &run_id));
        }
        state.runs.insert(run_id, run);
        Ok(())
    }

    async fn get_job(&self, job_id: &JobId) -> ForgeResult<JobRun> {
        let state = self.state.read().expect("memory store lock");
        for jobs in state.jobs.values() {
            if let Some(job) = jobs.get(job_id.as_str()) {
                return Ok(job.clone());
            }
        }
        Err(not_found("job", job_id.as_str()))
    }

    async fn list_jobs_for_run(&self, run_id: &RunId) -> ForgeResult<Vec<JobRun>> {
        let state = self.state.read().expect("memory store lock");
        let jobs = state
            .jobs
            .get(run_id.as_str())
            .ok_or_else(|| not_found("run", run_id.as_str()))?;
        let mut list: Vec<JobRun> = jobs.values().cloned().collect();
        list.sort_by(|a, b| {
            a.created_at_ms
                .cmp(&b.created_at_ms)
                .then(a.id.as_str().cmp(b.id.as_str()))
        });
        Ok(list)
    }

    async fn update_job(&self, job: JobRun) -> ForgeResult<()> {
        let mut state = self.state.write().expect("memory store lock");
        let run_id = job.run_id.to_string();
        let job_id = job.id.to_string();
        let jobs = state
            .jobs
            .get_mut(&run_id)
            .ok_or_else(|| not_found("run", &run_id))?;
        if !jobs.contains_key(&job_id) {
            return Err(not_found("job", &job_id));
        }
        jobs.insert(job_id, job);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::{JobStatus, RunStatus};

    fn sample_run() -> (Run, Vec<JobRun>) {
        let run = Run::new(RunId::new("r1"), "demo", "manual");
        let job = JobRun::new(
            JobId::new("r1::build"),
            RunId::new("r1"),
            "build",
            "Build",
            vec![],
            Some(5),
            Default::default(),
            vec![],
        );
        (run, vec![job])
    }

    #[tokio::test]
    async fn create_get_update_roundtrip() {
        let store = MemoryStore::new();
        let (run, jobs) = sample_run();
        let run_id = run.id.clone();
        store.create_run(run.clone(), jobs).await.expect("create");

        let fetched = store.get_run(&run_id).await.expect("get");
        assert_eq!(fetched.workflow_name, "demo");
        assert_eq!(fetched.status, RunStatus::Pending);

        let mut run2 = fetched;
        run2.status = RunStatus::Completed;
        store.update_run(run2.clone()).await.expect("update");
        assert_eq!(
            store.get_run(&run_id).await.expect("get2").status,
            RunStatus::Completed
        );

        let mut job = store.get_job(&JobId::new("r1::build")).await.expect("job");
        job.status = JobStatus::Completed;
        store.update_job(job).await.expect("update job");
        let jobs = store.list_jobs_for_run(&run_id).await.expect("list");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].status, JobStatus::Completed);
    }

    #[tokio::test]
    async fn missing_entities_error() {
        let store = MemoryStore::new();
        assert!(store.get_run(&RunId::new("nope")).await.is_err());
        assert!(store.get_job(&JobId::new("nope")).await.is_err());
        let (run, jobs) = sample_run();
        store.create_run(run, jobs).await.unwrap();
        assert!(store
            .update_run(Run::new(RunId::new("other"), "x", "manual"))
            .await
            .is_err());
    }
}
