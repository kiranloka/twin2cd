//! Remote execution over SSH or agents (stub).

use crate::traits::{ExecutionError, ExecutionOutcome, Executor};
use async_trait::async_trait;
use forge_core::ExecutionContext;

#[derive(Debug, Default)]
pub struct RemoteExecutor;

#[async_trait]
impl Executor for RemoteExecutor {
    async fn execute(&self, _context: ExecutionContext) -> ExecutionOutcome {
        Err(ExecutionError::Failed(
            "remote executor not yet implemented".into(),
        ))
    }

    async fn cancel(&self, _execution_id: &str) -> Result<(), ExecutionError> {
        Ok(())
    }
}