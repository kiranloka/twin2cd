use thiserror::Error;

#[derive(Debug, Error)]
pub enum ForgeError {
    #[error("workflow error: {0}")]
    Workflow(String),
    #[error("execution error: {0}")]
    Execution(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("not found: {0}")]
    NotFound(String),
}

pub type ForgeResult<T> = Result<T, ForgeError>;
