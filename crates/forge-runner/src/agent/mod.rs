use forge_protocol::v1::JobAssignment;
use forge_protocol::v1::{
    ControllerFrame, JobCompletionReport, JobLeaseRenewal, RunnerFrame, RunnerHeartbeat,
    RunnerRegistration, runner_control_client::RunnerControlClient, runner_frame,
};
use std::{io, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::mpsc,
    task::JoinHandle,
    time,
};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, metadata::MetadataValue};
use tracing::{error, info, warn};

pub async fn run() {
    let runner_id = std::env::var("FORGE_RUNNER_ID").unwrap_or_else(|_| "local-runner".into());
    let token = std::env::var("FORGE_RUNNER_TOKEN").unwrap_or_default();
    let endpoint = std::env::var("FORGE_CONTROLLER_GRPC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8081".into());
    if token.is_empty() {
        error!("FORGE_RUNNER_TOKEN is required; register this runner first");
        return;
    }
    loop {
        if let Err(error) = connect_and_run(&endpoint, &token, &runner_id).await {
            warn!(%error, "runner stream ended; reconnecting");
        }
        time::sleep(Duration::from_secs(2)).await;
    }
}

async fn connect_and_run(
    endpoint: &str,
    token: &str,
    runner_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut client = RunnerControlClient::connect(endpoint.to_owned()).await?;
    let (sender, receiver) = mpsc::channel(32);
    let mut request = Request::new(ReceiverStream::new(receiver));
    request.metadata_mut().insert(
        "authorization",
        MetadataValue::try_from(format!("Bearer {token}"))?,
    );
    sender.send(registration_frame(runner_id)).await?;
    let response = client.stream(request).await?;
    info!(runner_id, "connected to Forge controller");
    let mut inbound = response.into_inner();
    let mut heartbeat = time::interval(Duration::from_secs(10));
    let mut active: Option<JoinHandle<_>> = None;
    let mut active_assignment = None;
    loop {
        tokio::select! {
            frame = inbound.message() => {
                let Some(frame) = frame? else { break };
                if let Some(ControllerFrame { payload: Some(forge_protocol::v1::controller_frame::Payload::Assignment(assignment)) }) = Some(frame) {
                    if active.is_some() { continue; }
                    let task_assignment = assignment.clone();
                    let task_sender = sender.clone();
                    active_assignment = Some(assignment);
                    active = Some(tokio::spawn(async move {
                        let exit_code = execute_command(&task_assignment, task_sender).await;
                        (task_assignment, exit_code)
                    }));
                }
            }
            _ = heartbeat.tick() => {
                sender.send(RunnerFrame { payload: Some(runner_frame::Payload::Heartbeat(RunnerHeartbeat { active_jobs: if active.is_some() { 1 } else { 0 }, status: if active.is_some() { "busy" } else { "idle" }.into() })) }).await?;
                if let Some(assignment) = active_assignment.as_ref() {
                    sender.send(RunnerFrame { payload: Some(runner_frame::Payload::LeaseRenewal(JobLeaseRenewal {
                        job_id: assignment.job_id.clone(), idempotency_key: assignment.idempotency_key.clone(), extend_secs: assignment.lease_ttl_secs,
                    })) }).await?;
                }
            }
            result = async { active.as_mut().expect("active task selected").await }, if active.is_some() => {
                active = None;
                let (assignment, exit_code) = result?;
                sender.send(RunnerFrame { payload: Some(runner_frame::Payload::Completion(JobCompletionReport {
                    job_id: assignment.job_id, idempotency_key: assignment.idempotency_key, exit_code,
                })) }).await?;
                active_assignment = None;
            }
        }
    }
    Ok(())
}

async fn execute_command(assignment: &JobAssignment, sender: mpsc::Sender<RunnerFrame>) -> i32 {
    if assignment.command.trim().is_empty() {
        let _ = send_log(
            &sender,
            &assignment.run_id,
            "stderr",
            "Job has no executable shell steps",
        )
        .await;
        return 1;
    }
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(&assignment.command)
        .current_dir(std::env::current_dir().unwrap_or_default())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let _ = send_log(&sender, &assignment.run_id, "stderr", &error.to_string()).await;
            return 1;
        }
    };
    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");
    let run_id = assignment.run_id.clone();
    let out_sender = sender.clone();
    let out_run = run_id.clone();
    let out_task = tokio::spawn(async move {
        forward_lines(BufReader::new(stdout), out_sender, out_run, "stdout").await
    });
    let err_task = tokio::spawn(async move {
        forward_lines(BufReader::new(stderr), sender, run_id, "stderr").await
    });
    let status = child.wait().await;
    let _ = out_task.await;
    let _ = err_task.await;
    status.ok().and_then(|status| status.code()).unwrap_or(1)
}

async fn forward_lines<R: tokio::io::AsyncRead + Unpin>(
    reader: BufReader<R>,
    sender: mpsc::Sender<RunnerFrame>,
    run_id: String,
    stream: &'static str,
) -> io::Result<()> {
    let mut lines = reader.lines();
    while let Some(line) = lines.next_line().await? {
        let line = if line.len() > 16_384 {
            line.chars().take(16_384).collect()
        } else {
            line
        };
        send_log(&sender, &run_id, stream, &line)
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "runner stream closed"))?;
    }
    Ok(())
}

async fn send_log(
    sender: &mpsc::Sender<RunnerFrame>,
    run_id: &str,
    stream: &str,
    line: &str,
) -> Result<(), mpsc::error::SendError<RunnerFrame>> {
    sender
        .send(RunnerFrame {
            payload: Some(runner_frame::Payload::Log(forge_protocol::v1::RunLogLine {
                run_id: run_id.into(),
                stream: stream.into(),
                line: line.into(),
            })),
        })
        .await
}

fn registration_frame(runner_id: &str) -> RunnerFrame {
    RunnerFrame {
        payload: Some(runner_frame::Payload::Registration(RunnerRegistration {
            runner_id: runner_id.into(),
            name: runner_id.into(),
            platform: std::env::consts::OS.into(),
            architecture: std::env::consts::ARCH.into(),
            labels: vec!["self-hosted".into()],
            executors: vec!["native".into()],
            cpu: 1,
            memory_mb: 1024,
            max_concurrent_jobs: 1,
        })),
    }
}
