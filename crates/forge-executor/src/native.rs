use crate::traits::{ExecutionError, ExecutionOutcome, Executor};
use async_trait::async_trait;
use forge_core::{ExecutionContext, ExecutionResult};
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::process::Command;
use tokio::sync::oneshot;

/// Native process executor with cooperative cancellation.
///
/// A running execution can be cancelled via [`Executor::cancel`]; the child
/// process is killed through `kill_on_drop` when the execute future is dropped
/// or when the in-band cancel signal fires.
#[derive(Debug, Default)]
pub struct NativeExecutor {
    active: Mutex<HashMap<String, oneshot::Sender<()>>>,
}

impl NativeExecutor {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Executor for NativeExecutor {
    async fn execute(&self, context: ExecutionContext) -> ExecutionOutcome {
        let exec_id = context.execution_id.to_string();
        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
        self.active
            .lock()
            .expect("native executor lock")
            .insert(exec_id.clone(), cancel_tx);

        let result = run_child(&context, cancel_rx).await;

        self.active
            .lock()
            .expect("native executor lock")
            .remove(&exec_id);

        result
    }

    async fn cancel(&self, execution_id: &str) -> Result<(), ExecutionError> {
        if let Some(tx) = self
            .active
            .lock()
            .expect("native executor lock")
            .remove(execution_id)
        {
            let _ = tx.send(());
        }
        Ok(())
    }
}

async fn run_child(
    context: &ExecutionContext,
    cancel_rx: oneshot::Receiver<()>,
) -> ExecutionOutcome {
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(&context.command)
        .current_dir(&context.working_directory)
        .envs(&context.environment)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let child = command
        .spawn()
        .map_err(|e| ExecutionError::Failed(format!("spawn failed: {e}")))?;

    let wait = child.wait_with_output();
    tokio::pin!(wait);

    tokio::select! {
        result = &mut wait => {
            let output = result.map_err(|e| ExecutionError::Failed(e.to_string()))?;
            Ok(ExecutionResult {
                exit_code: output.status.code().unwrap_or(1),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
        }
        _ = cancel_rx => {
            // Dropping `wait` drops the child; `kill_on_drop(true)` reaps it.
            Err(ExecutionError::Cancelled)
        }
    }
}
