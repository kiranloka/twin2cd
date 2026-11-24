use crate::{auth::hash_token, state::AppState};
use forge_protocol::v1::{
    runner_control_server::{RunnerControl, RunnerControlServer},
    runner_frame, ControllerFrame, JobAssignment, OperationResult, RegistrationAccepted,
    RunnerFrame,
};
use forge_protocol::DEFAULT_JOB_LEASE_TTL_SECS;
use futures_util::Stream;
use std::{pin::Pin, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};

#[derive(Clone)]
struct RunnerControlService {
    state: AppState,
}

type ConnectStream = Pin<Box<dyn Stream<Item = Result<ControllerFrame, Status>> + Send>>;

#[tonic::async_trait]
impl RunnerControl for RunnerControlService {
    type StreamStream = ConnectStream;

    async fn stream(
        &self,
        request: Request<Streaming<RunnerFrame>>,
    ) -> Result<Response<Self::StreamStream>, Status> {
        let token = request
            .metadata()
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or_else(|| Status::unauthenticated("runner bearer token required"))?;
        let record = forge_storage::repos::tokens::find_by_hash(
            self.state.storage.pool(),
            &hash_token(token),
        )
        .await
        .map_err(|_| Status::internal("token lookup failed"))?
        .ok_or_else(|| Status::unauthenticated("invalid or expired runner token"))?;
        let runner_id = record
            .runner_id
            .clone()
            .filter(|_| record.role == forge_storage::Role::Runner)
            .ok_or_else(|| Status::permission_denied("token is not scoped to a runner"))?;

        let (sender, receiver) = mpsc::channel(32);
        let state = self.state.clone();
        let mut inbound = request.into_inner();
        tokio::spawn(async move {
            let first = match inbound.message().await {
                Ok(Some(frame)) => frame,
                Ok(None) | Err(_) => return,
            };
            let Some(runner_frame::Payload::Registration(registration)) = first.payload else {
                let _ = sender
                    .send(Err(Status::failed_precondition(
                        "first runner frame must register capabilities",
                    )))
                    .await;
                return;
            };
            if registration.runner_id != runner_id {
                let _ = sender
                    .send(Err(Status::permission_denied(
                        "registration does not match runner token",
                    )))
                    .await;
                return;
            }
            let labels = match serde_json::to_string(&registration.labels) {
                Ok(value) => value,
                Err(_) => return,
            };
            let executors = match serde_json::to_string(&registration.executors) {
                Ok(value) => value,
                Err(_) => return,
            };
            let capacity = serde_json::json!({
                "cpu": registration.cpu,
                "memory_mb": registration.memory_mb,
                "max_concurrent_jobs": registration.max_concurrent_jobs,
            })
            .to_string();
            match forge_storage::repos::runners::upsert(
                state.storage.pool(),
                &record.organization_id,
                &runner_id,
                &registration.name,
                &registration.platform,
                &registration.architecture,
                &labels,
                &executors,
                &capacity,
                registration.max_concurrent_jobs,
            )
            .await
            {
                Ok(true) => {}
                Ok(false) => {
                    let _ = sender
                        .send(Err(Status::permission_denied(
                            "runner organization mismatch",
                        )))
                        .await;
                    return;
                }
                Err(_) => {
                    let _ = sender
                        .send(Err(Status::internal("runner registration failed")))
                        .await;
                    return;
                }
            }
            if sender
                .send(Ok(ControllerFrame {
                    payload: Some(forge_protocol::v1::controller_frame::Payload::Registration(
                        RegistrationAccepted {
                            runner_id: runner_id.clone(),
                            heartbeat_interval_secs: 15,
                        },
                    )),
                }))
                .await
                .is_err()
            {
                return;
            }

            let mut poll = tokio::time::interval(Duration::from_secs(3));
            loop {
                tokio::select! {
                    incoming = inbound.message() => {
                        let frame = match incoming {
                            Ok(Some(frame)) => frame,
                            Ok(None) | Err(_) => break,
                        };
                        let Some(payload) = frame.payload else { continue };
                        let result = match payload {
                            runner_frame::Payload::Heartbeat(_) => {
                                forge_storage::repos::runners::record_heartbeat(
                                    state.storage.pool(), &record.organization_id, &runner_id,
                                ).await.map_err(|_| Status::internal("heartbeat update failed"))
                            }
                            runner_frame::Payload::LeaseRenewal(renewal) => {
                                forge_storage::repos::executions::renew_job_lease(
                                    state.storage.pool(), &record.organization_id, &runner_id,
                                    &renewal.job_id, &renewal.idempotency_key,
                                    renewal.extend_secs.clamp(1, 300),
                                ).await.map_err(|_| Status::internal("lease renewal failed"))
                            }
                            runner_frame::Payload::Completion(completion) => {
                                forge_storage::repos::executions::complete_job(
                                    state.storage.pool(), &record.organization_id, &runner_id,
                                    &completion.job_id, &completion.idempotency_key,
                                    completion.exit_code,
                                ).await.map_err(|_| Status::internal("completion report failed"))
                            }
                            runner_frame::Payload::Log(log) => {
                                forge_storage::repos::executions::append_run_log(
                                    state.storage.pool(), &record.organization_id, &runner_id,
                                    &log.run_id, &log.stream, &log.line,
                                ).await.map_err(|_| Status::internal("log append failed"))
                            }
                            runner_frame::Payload::Registration(_) => {
                                Err(Status::invalid_argument("registration is only valid as the first frame"))
                            }
                        };
                        match result {
                            Ok(ok) => {
                                if sender.send(Ok(ControllerFrame {
                                    payload: Some(forge_protocol::v1::controller_frame::Payload::Result(
                                        OperationResult { ok, message: String::new() },
                                    )),
                                })).await.is_err() { break; }
                            }
                            Err(status) => {
                                if sender.send(Err(status)).await.is_err() { break; }
                            }
                        }
                    }
                    _ = poll.tick() => {
                        let mut bytes = [0u8; 16];
                        if getrandom::getrandom(&mut bytes).is_err() { continue; }
                        let idempotency_key = hex::encode(bytes);
                        match forge_storage::repos::executions::claim_next_job(
                            state.storage.pool(), &record.organization_id, &runner_id,
                            DEFAULT_JOB_LEASE_TTL_SECS, &idempotency_key,
                        ).await {
                            Ok(Some(job)) => {
                                let assignment = JobAssignment {
                                    job_id: job.job_id,
                                    run_id: job.run_id,
                                    name: job.name,
                                    command: job.command,
                                    lease_ttl_secs: DEFAULT_JOB_LEASE_TTL_SECS,
                                    idempotency_key,
                                };
                                if sender.send(Ok(ControllerFrame {
                                    payload: Some(forge_protocol::v1::controller_frame::Payload::Assignment(assignment)),
                                })).await.is_err() { break; }
                            }
                            Ok(None) => {}
                            Err(error) => tracing::error!(%error, %runner_id, "job assignment query failed"),
                        }
                    }
                }
            }
            let _ = forge_storage::repos::runners::set_offline(
                state.storage.pool(),
                &record.organization_id,
                &runner_id,
            )
            .await;
        });
        Ok(Response::new(Box::pin(ReceiverStream::new(receiver))))
    }
}

pub async fn serve(
    state: AppState,
    address: std::net::SocketAddr,
) -> Result<(), tonic::transport::Error> {
    tonic::transport::Server::builder()
        .add_service(RunnerControlServer::new(RunnerControlService { state }))
        .serve(address)
        .await
}
