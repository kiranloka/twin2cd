//! Run driver: needs-gated job scheduling and terminal-state resolution.

use crate::step;
use crate::EngineShared;
use forge_common::RunId;
use forge_core::{JobStatus, RunStatus};
use forge_workflow::CompiledWorkflow;
use std::collections::HashMap;
use tokio_util::sync::CancellationToken;

/// Drive a run to a terminal state.
pub(crate) async fn drive_run(
    shared: EngineShared,
    run_id: RunId,
    compiled: CompiledWorkflow,
    token: CancellationToken,
) {
    let result = drive_inner(&shared, &run_id, &compiled, &token).await;

    match shared.store.get_run(&run_id).await {
        Ok(mut run) if !run.status.is_terminal() => {
            let status = result.unwrap_or(RunStatus::Completed);
            run.status = status;
            run.finished_at_ms = Some(forge_core::now_ms());
            match status {
                RunStatus::Failed => run.error = Some("one or more jobs failed".into()),
                RunStatus::Cancelled => run.error = Some("run cancelled".into()),
                _ => {}
            }
            if let Err(e) = shared.store.update_run(run).await {
                tracing::error!(%run_id, error = %e, "failed to finalize run");
            }
        }
        Ok(_) => {}
        Err(e) => tracing::error!(%run_id, error = %e, "failed to load run for finalize"),
    }

    shared
        .cancellations
        .lock()
        .expect("engine cancel lock")
        .remove(run_id.as_str());

    tracing::info!(%run_id, "run driver finished");
}

async fn drive_inner(
    shared: &EngineShared,
    run_id: &RunId,
    compiled: &CompiledWorkflow,
    token: &CancellationToken,
) -> Option<RunStatus> {
    let mut statuses: HashMap<String, JobStatus> = compiled
        .document
        .jobs
        .keys()
        .map(|k| (k.clone(), JobStatus::Pending))
        .collect();

    // Reverse edges: job_key → dependents.
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
    for (key, spec) in &compiled.document.jobs {
        for need in &spec.needs {
            dependents
                .entry(need.clone())
                .or_default()
                .push(key.clone());
        }
    }

    let mut join_set: tokio::task::JoinSet<(String, JobStatus)> = tokio::task::JoinSet::new();

    // Seed jobs with no dependencies.
    let roots: Vec<String> = compiled
        .document
        .jobs
        .iter()
        .filter(|(_, spec)| spec.needs.is_empty())
        .map(|(k, _)| k.clone())
        .collect();
    for key in roots {
        spawn_job(
            shared,
            run_id,
            compiled,
            token,
            &mut join_set,
            key,
            &mut statuses,
        );
    }

    while let Some(joined) = join_set.join_next().await {
        let (key, status) = match joined {
            Ok(pair) => pair,
            Err(e) => {
                tracing::error!(error = %e, "job task panicked");
                continue;
            }
        };
        tracing::info!(job = %key, ?status, "job finished");
        statuses.insert(key.clone(), status);

        if token.is_cancelled() {
            // Every active job observes the shared cancellation token and
            // asks its executor to stop. Drain them so executor cleanup and
            // terminal job persistence finish before marking queued jobs.
            while let Some(joined) = join_set.join_next().await {
                if let Ok((active_key, active_status)) = joined {
                    statuses.insert(active_key, active_status);
                }
            }
            cancel_remaining(compiled, &mut statuses, run_id, shared).await;
            return Some(RunStatus::Cancelled);
        }

        // Propagate readiness / skip cascade to dependents.
        let mut ready: Vec<String> = Vec::new();
        sweep_pending(
            compiled,
            &dependents,
            &mut statuses,
            &mut ready,
            run_id,
            shared,
        )
        .await;

        for key in ready {
            spawn_job(
                shared,
                run_id,
                compiled,
                token,
                &mut join_set,
                key,
                &mut statuses,
            );
        }

        if statuses.values().all(|s| s.is_terminal()) {
            break;
        }
    }

    // Drain remaining tasks (after natural completion the set is empty).
    while let Some(joined) = join_set.join_next().await {
        if let Ok((key, status)) = joined {
            statuses.insert(key, status);
        }
    }

    if token.is_cancelled() {
        cancel_remaining(compiled, &mut statuses, run_id, shared).await;
        return Some(RunStatus::Cancelled);
    }

    if statuses.values().any(|s| *s == JobStatus::Failed) {
        return Some(RunStatus::Failed);
    }
    if statuses.values().any(|s| !s.is_terminal()) {
        tracing::error!(%run_id, "driver exited with non-terminal jobs");
        return Some(RunStatus::Failed);
    }
    Some(RunStatus::Completed)
}

fn spawn_job(
    shared: &EngineShared,
    run_id: &RunId,
    compiled: &CompiledWorkflow,
    token: &CancellationToken,
    join_set: &mut tokio::task::JoinSet<(String, JobStatus)>,
    key: String,
    statuses: &mut HashMap<String, JobStatus>,
) {
    statuses.insert(key.clone(), JobStatus::Ready);
    let shared = shared.clone();
    let run_id = run_id.clone();
    let compiled = compiled.clone();
    let token = token.clone();
    let job_key = key;
    join_set.spawn(async move {
        let status = step::execute_job(&shared, &run_id, &compiled, &job_key, &token).await;
        (job_key, status)
    });
}

/// Evaluate every pending job: mark Ready (spawn) or Skipped (cascade).
async fn sweep_pending(
    compiled: &CompiledWorkflow,
    dependents: &HashMap<String, Vec<String>>,
    statuses: &mut HashMap<String, JobStatus>,
    ready: &mut Vec<String>,
    run_id: &RunId,
    shared: &EngineShared,
) {
    let _ = dependents; // full sweep over pending jobs covers all edges
    let mut changed = true;
    while changed {
        changed = false;
        let pending: Vec<String> = statuses
            .iter()
            .filter(|(_, s)| **s == JobStatus::Pending)
            .map(|(k, _)| k.clone())
            .collect();

        for dep in pending {
            let spec = match compiled.document.jobs.get(&dep) {
                Some(s) => s,
                None => continue,
            };
            let all_terminal = spec
                .needs
                .iter()
                .all(|n| statuses.get(n).map(|s| s.is_terminal()).unwrap_or(false));
            if !all_terminal {
                continue;
            }
            let all_ok = spec
                .needs
                .iter()
                .all(|n| statuses.get(n) == Some(&JobStatus::Completed));

            if all_ok {
                statuses.insert(dep.clone(), JobStatus::Ready);
                if let Err(e) =
                    update_job_status(shared, run_id, &dep, JobStatus::Ready, None).await
                {
                    tracing::error!(job = %dep, error = %e, "failed to mark ready");
                }
                ready.push(dep);
                changed = true;
            } else {
                statuses.insert(dep.clone(), JobStatus::Skipped);
                if let Err(e) =
                    update_job_status(shared, run_id, &dep, JobStatus::Skipped, None).await
                {
                    tracing::error!(job = %dep, error = %e, "failed to mark skipped");
                }
                changed = true;
            }
        }
    }
}

async fn cancel_remaining(
    compiled: &CompiledWorkflow,
    statuses: &mut HashMap<String, JobStatus>,
    run_id: &RunId,
    shared: &EngineShared,
) {
    for key in compiled.document.jobs.keys() {
        let current = statuses.get(key).copied().unwrap_or(JobStatus::Pending);
        if !current.is_terminal() {
            statuses.insert(key.clone(), JobStatus::Cancelled);
            if let Err(e) = update_job_status(
                shared,
                run_id,
                key,
                JobStatus::Cancelled,
                Some("run cancelled"),
            )
            .await
            {
                tracing::error!(job = %key, error = %e, "failed to mark cancelled");
            }
        }
    }
}

async fn update_job_status(
    shared: &EngineShared,
    run_id: &RunId,
    job_key: &str,
    status: JobStatus,
    error: Option<&str>,
) -> forge_core::ForgeResult<()> {
    let job_id = forge_common::JobId::new(format!("{run_id}::{job_key}"));
    let mut job = shared.store.get_job(&job_id).await?;
    job.status = status;
    if let Some(err) = error {
        job.error = Some(err.to_string());
    }
    match status {
        JobStatus::Running => {
            job.started_at_ms = Some(forge_core::now_ms());
        }
        s if s.is_terminal() => {
            job.finished_at_ms = Some(forge_core::now_ms());
        }
        _ => {}
    }
    shared.store.update_job(job).await
}
