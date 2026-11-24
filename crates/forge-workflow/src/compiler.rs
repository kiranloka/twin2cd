use crate::graph::WorkflowGraph;
use crate::validator::WorkflowValidator;
use forge_core::{ForgeResult, WorkflowDocument};

#[derive(Debug, Clone)]
pub struct CompiledWorkflow {
    pub document: WorkflowDocument,
    pub graph: WorkflowGraph,
}

#[derive(Debug, Default)]
pub struct WorkflowCompiler;

impl WorkflowCompiler {
    pub fn compile(document: WorkflowDocument) -> ForgeResult<CompiledWorkflow> {
        WorkflowValidator::validate(&document)?;
        let graph = WorkflowGraph::from_document(&document)?;
        Ok(CompiledWorkflow { document, graph })
    }
}