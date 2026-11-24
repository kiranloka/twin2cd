//! Job and step execution: env layering, expressions, timeouts, cancel.

use crate::EngineShared;
use forge_common::{ExecutionId, JobId, RunId};
use forge_core::{ExecutionContext, ExecutorKind, JobRun, JobStatus, StepStatus};
use forge_executor::{ExecutionError, ExecutionOutcome, Executor};
use forge_workflow::{evaluate_template, CompiledWorkflow, ExpressionContext};
use std::collections::BTreeMap;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Execute all steps of a job. Returns the terminal [`JobStatus`].
pub(crate) async fn execute_job(
    shared: &EngineShared,
    run_id: &RunId,
    compiled: &CompiledWorkflow,
    job_key: &str,
    token: &CancellationToken,
) -> JobStatus {
    let job_id = JobId::new(format!("{run_id}::{job_key}"));
    let spec = match compiled.document.jobs.get(job_key) {
        Some(s) => s,
        None => {
            tracing::error!(job = %job_key, "job spec missing from compiled workflow");
            return JobStatus::Failed;
        }
    };

    // Load job record.
    let mut job = match shared.store.get_job(&job_id).await {
        Ok(j) => j,
        Err(e) => {
            tracing::error!(%job_id, error = %e, "job record missing");
            return JobStatus::Failed;
        }
    };

    if token.is_cancelled() {
        return finish_job(
            shared,
            &mut job,
            JobStatus::Cancelled,
            Some("run cancelled"),
        )
        .await;
    }

    let permit = tokio::select! {
        permit = shared.job_semaphore.clone().acquire_owned() => match permit {
            Ok(permit) => permit,
            Err(_) => return finish_job(shared, &mut job, JobStatus::Failed, Some("job scheduler is closed")).await,
        },
        _ = token.cancelled() => return finish_job(shared, &mut job, JobStatus::Cancelled, Some("run cancelled")).await,
    };

    job.status = JobStatus::Running;
    job.started_at_ms = Some(forge_core::now_ms());
    if let Err(e) = shared.store.update_job(job.clone()).await {
        tracing::error!(%job_id, error = %e, "failed to mark job running");
        return JobStatus::Failed;
    }
    let _permit = permit;
    tracing::info!(%job_id, job = %job_key, "job started");

    // Workspace
    let workspace = shared
        .config
        .workspace_root
        .join(sanitize_path(run_id.as_str()))
        .join(sanitize_path(job_key));
    if let Err(e) = tokio::fs::create_dir_all(&workspace).await {
        let msg = format!("create workspace: {e}");
        return finish_job(shared, &mut job, JobStatus::Failed, Some(&msg)).await;
    }

    // ---- env / expression context ----------------------------------------
    let secrets_snapshot = shared.secrets.snapshot();
    let mut expr = ExpressionContext {
        env: BTreeMap::new(),
        secrets: secrets_snapshot,
        variables: compiled.document.variables.clone(),
    };

    // Workflow-level environment (evaluated against secrets/variables).
    let mut base_env: BTreeMap<String, String> = BTreeMap::new();
    for (k, v) in &compiled.document.environment {
        base_env.insert(k.clone(), evaluate_template(v, &expr));
    }
    expr.env = base_env.clone();

    // Job-level env.
    let mut job_env = base_env.clone();
    for (k, v) in &spec.env {
        job_env.insert(k.clone(), evaluate_template(v, &expr));
    }
    expr.env = job_env.clone();

    // Standard variables.
    job_env.insert("FORGE_RUN_ID".into(), run_id.to_string());
    job_env.insert("FORGE_JOB_ID".into(), job_id.to_string());
    job_env.insert("FORGE_JOB_KEY".into(), job_key.to_string());

    job.env = job_env.clone();
    if let Err(e) = shared.store.update_job(job.clone()).await {
        tracing::error!(%job_id, error = %e, "failed to persist job env");
    }

    // ---- timeout budget ----------------------------------------------------
    let job_timeout_minutes = job
        .timeout_minutes
        .unwrap_or(shared.config.default_job_timeout_minutes)
        .max(1);
    let job_timeout_minutes = job_timeout_minutes.min(7 * 24 * 60);
    let job_deadline =
        tokio::time::Instant::now() + Duration::from_secs(job_timeout_minutes.saturating_mul(60));

    // ---- run steps ---------------------------------------------------------
    let step_count = job.steps.len();

    for index in 0..step_count {
        if token.is_cancelled() {
            mark_remaining_steps(&mut job, index, StepStatus::Cancelled);
            return finish_job(
                shared,
                &mut job,
                JobStatus::Cancelled,
                Some("run cancelled"),
            )
            .await;
        }

        let remaining = job_deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            mark_remaining_steps(&mut job, index, StepStatus::Skipped);
            let msg = format!("job timed out after {job_timeout_minutes} minutes");
            return finish_job(shared, &mut job, JobStatus::Failed, Some(&msg)).await;
        }

        let step_spec = &spec.steps[index];
        let mut step = job.steps[index].clone();
        step.status = StepStatus::Running;
        step.started_at_ms = Some(forge_core::now_ms());
        job.steps[index] = step.clone();
        if let Err(e) = shared.store.update_job(job.clone()).await {
            tracing::error!(%job_id, step = index, error = %e, "failed to mark step running");
        }
        tracing::info!(%job_id, step = index, name = %step.name, "step started");

        // Step env layering.
        let mut step_env = job_env.clone();
        expr.env = job_env.clone();
        for (k, v) in &step_spec.env {
            step_env.insert(k.clone(), evaluate_template(v, &expr));
        }
        expr.env = step_env.clone();

        // Resolve command.
        let command = match &step_spec.run {
            Some(run_template) => evaluate_template(run_template, &expr),
            None => {
                let msg = if step_spec.uses.is_some() {
                    format!(
                        "step '{}' uses actions, which are not yet supported",
                        step.name
                    )
                } else {
                    format!("step '{}' has no run command", step.name)
                };
                step.status = StepStatus::Failed;
                step.error = Some(msg.clone());
                step.finished_at_ms = Some(forge_core::now_ms());
                job.steps[index] = step;
                let _ = shared.store.update_job(job.clone()).await;
                return finish_job(shared, &mut job, JobStatus::Failed, Some(&msg)).await;
            }
        };

        // Step timeout = min(step override, remaining job budget).
        let step_timeout = step_spec
            .timeout_minutes
            .map(|m| Duration::from_secs(m.max(1).min(7 * 24 * 60).saturating_mul(60)))
            .unwrap_or(remaining)
            .min(remaining);

        let execution_id = ExecutionId::new(format!("{run_id}::{job_key}::step:{index}"));

        let context = ExecutionContext {
            execution_id: execution_id.clone(),
            command,
            working_directory: workspace.to_string_lossy().into_owned(),
            environment: step_env.clone(),
            image: spec.runs_on.image.clone(),
        };

        let outcome = run_step(
            shared,
            &spec.runs_on.executor,
            context,
            step_timeout,
            token,
            &execution_id,
        )
        .await;

        let finished = forge_core::now_ms();
        step.finished_at_ms = Some(finished);

        match outcome {
            Ok(result) if result.exit_code == 0 => {
                step.status = StepStatus::Completed;
                step.exit_code = Some(0);
                step.stdout = redact_output(&result.stdout, &expr.secrets);
                step.stderr = redact_output(&result.stderr, &expr.secrets);
                job.steps[index] = step;
                if let Err(e) = shared.store.update_job(job.clone()).await {
                    tracing::error!(%job_id, step = index, error = %e, "failed to persist step");
                }
                tracing::info!(%job_id, step = index, "step completed");
            }
            Ok(result) => {
                step.status = StepStatus::Failed;
                step.exit_code = Some(result.exit_code);
                step.stdout = redact_output(&result.stdout, &expr.secrets);
                step.stderr = redact_output(&result.stderr, &expr.secrets);
                step.error = Some(format!("exit code {}", result.exit_code));
                if !result.stderr.is_empty() {
                    tracing::warn!(
                        %job_id,
                        step = index,
                        stderr = %truncate(&step.stderr, 2000),
                        "step failed"
                    );
                }
                job.steps[index] = step;
                let _ = shared.store.update_job(job.clone()).await;
                let msg = format!(
                    "step '{}' failed with exit code {}",
                    spec.steps[index].name, result.exit_code
                );
                mark_remaining_steps(&mut job, index + 1, StepStatus::Skipped);
                return finish_job(shared, &mut job, JobStatus::Failed, Some(&msg)).await;
            }
            Err(ExecutionError::Cancelled) => {
                // Distinguish run-cancel from timeout: timeout path converts
                // Cancelled after calling cancel — check token.
                if token.is_cancelled() {
                    step.status = StepStatus::Cancelled;
                    step.error = Some("run cancelled".into());
                    job.steps[index] = step;
                    let _ = shared.store.update_job(job.clone()).await;
                    mark_remaining_steps(&mut job, index + 1, StepStatus::Cancelled);
                    return finish_job(
                        shared,
                        &mut job,
                        JobStatus::Cancelled,
                        Some("run cancelled"),
                    )
                    .await;
                }
                // Timed out (engine called executor.cancel without run cancel).
                step.status = StepStatus::TimedOut;
                step.error = Some(format!("step timed out after {}s", step_timeout.as_secs()));
                job.steps[index] = step;
                let _ = shared.store.update_job(job.clone()).await;
                mark_remaining_steps(&mut job, index + 1, StepStatus::Skipped);
                let msg = format!(
                    "step '{}' timed out after {} minute(s)",
                    spec.steps[index].name,
                    step_timeout.as_secs().div_ceil(60).max(1)
                );
                return finish_job(shared, &mut job, JobStatus::Failed, Some(&msg)).await;
            }
            Err(ExecutionError::Failed(e)) => {
                step.status = StepStatus::Failed;
                step.error = Some(e.clone());
                job.steps[index] = step;
                let _ = shared.store.update_job(job.clone()).await;
                mark_remaining_steps(&mut job, index + 1, StepStatus::Skipped);
                return finish_job(shared, &mut job, JobStatus::Failed, Some(&e)).await;
            }
        }
    }

    // Outer job timeout can also fire between steps via deadline check above;
    // if we got here all steps succeeded.
    finish_job(shared, &mut job, JobStatus::Completed, None).await
}

async fn run_step(
    shared: &EngineShared,
    kind: &ExecutorKind,
    context: ExecutionContext,
    timeout: Duration,
    token: &CancellationToken,
    execution_id: &ExecutionId,
) -> ExecutionOutcome {
    let executor: &dyn Executor = match kind {
        ExecutorKind::Native => shared.native.as_ref(),
        ExecutorKind::Container => match &shared.container {
            Some(c) => c.as_ref(),
            None => {
                return Err(ExecutionError::Failed(
                    "container executor unavailable (docker not reachable)".into(),
                ));
            }
        },
        ExecutorKind::Remote => {
            return Err(ExecutionError::Failed(
                "remote executor not yet implemented".into(),
            ));
        }
    };

    let exec_fut = executor.execute(context);
    tokio::pin!(exec_fut);

    tokio::select! {
        result = &mut exec_fut => result,
        _ = token.cancelled() => {
            let _ = executor.cancel(execution_id.as_str()).await;
            let _ = tokio::time::timeout(Duration::from_secs(5), &mut exec_fut).await;
            Err(ExecutionError::Cancelled)
        }
        _ = tokio::time::sleep(timeout) => {
            let _ = executor.cancel(execution_id.as_str()).await;
            let _ = tokio::time::timeout(Duration::from_secs(5), &mut exec_fut).await;
            Err(ExecutionError::Cancelled)
        }
    }
}

async fn finish_job(
    shared: &EngineShared,
    job: &mut JobRun,
    status: JobStatus,
    error: Option<&str>,
) -> JobStatus {
    job.status = status;
    if let Some(err) = error {
        job.error = Some(err.to_string());
    }
    if status.is_terminal() {
        job.finished_at_ms = Some(forge_core::now_ms());
    }
    if let Err(e) = shared.store.update_job(job.clone()).await {
        tracing::error!(job_id = %job.id, error = %e, "failed to persist job terminal state");
    }
    tracing::info!(job_id = %job.id, ?status, error = ?job.error, "job finished");
    status
}

fn mark_remaining_steps(job: &mut JobRun, from: usize, status: StepStatus) {
    for step in job.steps.iter_mut().skip(from) {
        if !step.status.is_terminal() {
            step.status = status;
            step.finished_at_ms = Some(forge_core::now_ms());
        }
    }
}

fn sanitize_path(s: &str) -> String {
    let mut out = String::new();
    for byte in s.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'-' {
            out.push(byte as char);
        } else {
            out.push_str(&format!("_{byte:02x}_"));
        }
    }
    out
}

fn redact_output(output: &str, secrets: &BTreeMap<String, String>) -> String {
    let mut values: Vec<&str> = secrets
        .values()
        .map(String::as_str)
        .filter(|s| !s.is_empty())
        .collect();
    values.sort_by_key(|s| std::cmp::Reverse(s.len()));
    values.into_iter().fold(output.to_string(), |text, secret| {
        text.replace(secret, "***")
    })
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}
