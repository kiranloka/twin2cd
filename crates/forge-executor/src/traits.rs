use async_trait::async_trait;
use forge_core::{ExecutionContext, ExecutionResult};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("execution failed: {0}")]
    Failed(String),
    #[error("execution cancelled")]
    Cancelled,
}

pub type ExecutionOutcome = Result<ExecutionResult, ExecutionError>;

#[async_trait]
pub trait Executor: Send + Sync {
    async fn execute(&self, context: ExecutionContext) -> ExecutionOutcome;

    async fn cancel(&self, execution_id: &str) -> Result<(), ExecutionError>;
}