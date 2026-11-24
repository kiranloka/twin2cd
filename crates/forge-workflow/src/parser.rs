use forge_core::{ForgeError, ForgeResult, WorkflowDocument};

#[derive(Debug, Default)]
pub struct WorkflowParser;

impl WorkflowParser {
    pub fn parse_str(yaml: &str) -> ForgeResult<WorkflowDocument> {
        serde_yaml::from_str(yaml).map_err(|e| ForgeError::Workflow(e.to_string()))
    }
}