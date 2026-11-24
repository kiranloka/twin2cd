use forge_common::RunId;
use forge_core::{JobStatus, RunStatus, StepStatus};
use forge_engine::{RunEngine, RunEngineConfig};
use forge_secrets::SecretsManager;
use forge_storage::{MemoryStore, RunStore};
use forge_workflow::{WorkflowCompiler, WorkflowParser};
use std::{sync::Arc, time::Duration};

fn setup(max_concurrent_jobs: usize) -> (RunEngine, Arc<MemoryStore>) {
    let store = Arc::new(MemoryStore::new());
    let engine = RunEngine::new(
        store.clone(),
        Arc::new(SecretsManager::new()),
        RunEngineConfig {
            max_concurrent_jobs,
            default_job_timeout_minutes: 1,
            workspace_root: std::env::temp_dir().join("forge-engine-integration"),
        },
    );
    (engine, store)
}

fn compile(yaml: &str) -> forge_workflow::CompiledWorkflow {
    WorkflowCompiler::compile(WorkflowParser::parse_str(yaml).expect("valid workflow"))
        .expect("workflow compiles")
}

async fn wait_terminal(store: &Arc<MemoryStore>, run_id: &RunId) -> forge_core::Run {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        let run = store.get_run(run_id).await.expect("run exists");
        if run.status.is_terminal() {
            return run;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "run did not finish: {run:?}"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn wait_job_running(store: &Arc<MemoryStore>, run_id: &RunId) {
    let job_id = forge_common::JobId::new(format!("{run_id}::work"));
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let job = store.get_job(&job_id).await.expect("job exists");
        if job.status == JobStatus::Running
            && job
                .steps
                .first()
                .is_some_and(|step| step.status == StepStatus::Running)
        {
            return;
        }
        assert!(tokio::time::Instant::now() < deadline, "job did not start");
        tokio::task::yield_now().await;
    }
}

#[tokio::test]
async fn successful_workflow_reaches_completed() {
    let (engine, store) = setup(2);
    let run_id = engine
        .trigger(
            compile(
                r#"
version: "1"
name: success
jobs:
  work:
    name: Work
    steps:
      - name: Print
        run: echo success
"#,
            ),
            "test",
        )
        .await
        .expect("trigger");
    let run = wait_terminal(&store, &run_id).await;
    assert_eq!(run.status, RunStatus::Completed);
    let job = store
        .get_job(&forge_common::JobId::new(format!("{run_id}::work")))
        .await
        .unwrap();
    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.steps[0].status, StepStatus::Completed);
}

#[tokio::test]
async fn failed_job_skips_dependents() {
    let (engine, store) = setup(2);
    let run_id = engine
        .trigger(
            compile(
                r#"
version: "1"
name: cascade
jobs:
  fail:
    name: Fail
    steps:
      - name: Exit
        run: exit 9
  dependent:
    name: Dependent
    needs: [fail]
    steps:
      - name: Should skip
        run: echo unexpected
"#,
            ),
            "test",
        )
        .await
        .expect("trigger");
    let run = wait_terminal(&store, &run_id).await;
    assert_eq!(run.status, RunStatus::Failed);
    let jobs = store.list_jobs_for_run(&run_id).await.unwrap();
    let skipped = jobs.iter().find(|job| job.job_key == "dependent").unwrap();
    assert_eq!(skipped.status, JobStatus::Skipped);
}

#[tokio::test(start_paused = true)]
async fn step_timeout_marks_run_failed() {
    let (engine, store) = setup(1);
    let run_id = engine
        .trigger(
            compile(
                r#"
version: "1"
name: timeout
jobs:
  work:
    name: Work
    steps:
      - name: Wait
        timeout_minutes: 1
        run: exec sleep 120
"#,
            ),
            "test",
        )
        .await
        .expect("trigger");
    wait_job_running(&store, &run_id).await;
    tokio::time::advance(Duration::from_secs(61)).await;
    tokio::time::resume();
    let run = wait_terminal(&store, &run_id).await;
    assert_eq!(run.status, RunStatus::Failed);
    let job = store
        .get_job(&forge_common::JobId::new(format!("{run_id}::work")))
        .await
        .unwrap();
    assert_eq!(job.steps[0].status, StepStatus::TimedOut);
}

#[tokio::test]
async fn cancellation_stops_running_workflow() {
    let (engine, store) = setup(1);
    let run_id = engine
        .trigger(
            compile(
                r#"
version: "1"
name: cancel
jobs:
  work:
    name: Work
    steps:
      - name: Wait
        run: sleep 30
"#,
            ),
            "test",
        )
        .await
        .expect("trigger");
    wait_job_running(&store, &run_id).await;
    engine.cancel(&run_id).await.expect("cancel requested");
    let run = wait_terminal(&store, &run_id).await;
    assert_eq!(run.status, RunStatus::Cancelled);
}

#[tokio::test]
async fn container_executor_runs_when_docker_is_available() {
    let Ok(executor) = forge_executor::ContainerExecutor::connect() else {
        return;
    };
    if !executor.ping().await {
        return;
    }
    drop(executor);

    let (engine, store) = setup(1);
    let run_id = engine
        .trigger(
            compile(
                r#"
version: "1"
name: container
jobs:
  work:
    name: Work
    runs_on:
      executor: container
      image: alpine:3.20
    steps:
      - name: Print
        run: echo container-ok
"#,
            ),
            "test",
        )
        .await
        .expect("trigger");
    let run = wait_terminal(&store, &run_id).await;
    assert_eq!(run.status, RunStatus::Completed);
}
