//! OCI container execution via the Docker Engine API (bollard).

use crate::traits::{ExecutionError, ExecutionOutcome, Executor};
use async_trait::async_trait;
use bollard::container::{
    Config, CreateContainerOptions, KillContainerOptions, LogsOptions, RemoveContainerOptions,
    StartContainerOptions, WaitContainerOptions,
};
use bollard::errors::Error as BollardError;
use bollard::models::HostConfig;
use bollard::Docker;
use forge_core::{ExecutionContext, ExecutionResult};
use futures_util::{StreamExt, TryStreamExt};
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::oneshot;

const WORKSPACE_MOUNT: &str = "/forge-workspace";

/// Container executor backed by a local Docker socket.
#[derive(Debug)]
pub struct ContainerExecutor {
    docker: Docker,
    /// execution_id → container_id for in-flight executions.
    active: Mutex<HashMap<String, (String, oneshot::Sender<()>)>>,
}

impl ContainerExecutor {
    pub fn connect() -> Result<Self, ExecutionError> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| ExecutionError::Failed(format!("docker connect: {e}")))?;
        Ok(Self {
            docker,
            active: Mutex::new(HashMap::new()),
        })
    }

    /// Check whether the connected Docker Engine is responding.
    pub async fn ping(&self) -> bool {
        self.docker.ping().await.is_ok()
    }

    fn container_name(execution_id: &str) -> String {
        let sanitized: String = execution_id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.') {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        format!("forge_{sanitized}")
    }

    async fn create_and_start(&self, context: &ExecutionContext) -> Result<String, ExecutionError> {
        let image = context.image.clone().ok_or_else(|| {
            ExecutionError::Failed("container executor requires runs_on.image".into())
        })?;

        let env: Vec<String> = context
            .environment
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();

        let config = Config::<String> {
            image: Some(image.clone()),
            cmd: Some(vec![
                "sh".to_string(),
                "-c".to_string(),
                context.command.clone(),
            ]),
            env: Some(env),
            working_dir: Some(WORKSPACE_MOUNT.to_string()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            host_config: Some(HostConfig {
                binds: Some(vec![format!(
                    "{}:{}",
                    context.working_directory, WORKSPACE_MOUNT
                )]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let name = Self::container_name(context.execution_id.as_str());
        let options = Some(CreateContainerOptions::<String> {
            name,
            platform: None,
        });

        let created = match self
            .docker
            .create_container(options.clone(), config.clone())
            .await
        {
            Ok(created) => created,
            Err(BollardError::DockerResponseServerError {
                status_code: 404, ..
            }) => {
                self.pull_image(&image).await?;
                self.docker
                    .create_container(options, config)
                    .await
                    .map_err(|e| ExecutionError::Failed(format!("create container: {e}")))?
            }
            Err(e) => return Err(ExecutionError::Failed(format!("create container: {e}"))),
        };

        let container_id = created.id;
        self.docker
            .start_container(&container_id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| ExecutionError::Failed(format!("start container: {e}")))?;

        Ok(container_id)
    }

    async fn pull_image(&self, image: &str) -> Result<(), ExecutionError> {
        let options = Some(bollard::image::CreateImageOptions {
            from_image: image,
            ..Default::default()
        });
        self.docker
            .create_image(options, None, None)
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| ExecutionError::Failed(format!("pull image '{image}': {e}")))?;
        Ok(())
    }

    async fn collect_logs(&self, container_id: &str) -> Result<(String, String), ExecutionError> {
        let options = Some(LogsOptions::<String> {
            stdout: true,
            stderr: true,
            follow: false,
            ..Default::default()
        });
        let logs: Vec<bollard::container::LogOutput> = self
            .docker
            .logs(container_id, options)
            .try_collect()
            .await
            .map_err(|e| ExecutionError::Failed(format!("read logs: {e}")))?;

        let mut stdout = String::new();
        let mut stderr = String::new();
        for entry in logs {
            match entry {
                bollard::container::LogOutput::StdOut { message }
                | bollard::container::LogOutput::Console { message } => {
                    stdout.push_str(&String::from_utf8_lossy(&message));
                }
                bollard::container::LogOutput::StdErr { message } => {
                    stderr.push_str(&String::from_utf8_lossy(&message));
                }
                bollard::container::LogOutput::StdIn { .. } => {}
            }
        }
        Ok((stdout, stderr))
    }

    async fn cleanup(&self, container_id: &str) {
        let _ = self
            .docker
            .remove_container(
                container_id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;
    }

    async fn kill(&self, container_id: &str) {
        let _ = self
            .docker
            .kill_container(
                container_id,
                Some(KillContainerOptions {
                    signal: "SIGKILL",
                    ..Default::default()
                }),
            )
            .await;
    }

    async fn wait_or_cancel(
        &self,
        container_id: &str,
        cancel_rx: oneshot::Receiver<()>,
    ) -> ExecutionOutcome {
        let mut wait = self
            .docker
            .wait_container(container_id, None::<WaitContainerOptions<String>>);

        tokio::select! {
            result = wait.next() => {
                let status = result
                    .ok_or_else(|| ExecutionError::Failed("container wait stream ended".into()))?
                    .map_err(|e| ExecutionError::Failed(format!("wait container: {e}")))?;
                let exit_code = status.status_code as i32;
                let (stdout, stderr) = self.collect_logs(container_id).await?;
                Ok(ExecutionResult {
                    exit_code,
                    stdout,
                    stderr,
                })
            }
            _ = cancel_rx => {
                self.kill(container_id).await;
                Err(ExecutionError::Cancelled)
            }
        }
    }
}

#[async_trait]
impl Executor for ContainerExecutor {
    async fn execute(&self, context: ExecutionContext) -> ExecutionOutcome {
        let exec_id = context.execution_id.to_string();
        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();

        let container_id = match self.create_and_start(&context).await {
            Ok(id) => id,
            Err(e) => {
                drop(cancel_tx);
                return Err(e);
            }
        };
        self.active
            .lock()
            .expect("container executor lock")
            .insert(exec_id.clone(), (container_id.clone(), cancel_tx));

        let result = self.wait_or_cancel(&container_id, cancel_rx).await;

        self.active
            .lock()
            .expect("container executor lock")
            .remove(&exec_id);
        self.cleanup(&container_id).await;
        result
    }

    async fn cancel(&self, execution_id: &str) -> Result<(), ExecutionError> {
        let active = {
            self.active
                .lock()
                .expect("container executor lock")
                .remove(execution_id)
        };
        if let Some((container_id, cancel_tx)) = active {
            let _ = cancel_tx.send(());
            self.kill(&container_id).await;
        }
        Ok(())
    }
}
