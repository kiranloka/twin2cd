use forge_core::{ForgeError, ForgeResult, WorkflowDocument};

#[derive(Debug, Default)]
pub struct WorkflowValidator;

impl WorkflowValidator {
    pub fn validate(document: &WorkflowDocument) -> ForgeResult<()> {
        if document.version.is_empty() {
            return Err(ForgeError::Workflow("version is required".into()));
        }
        if document.name.is_empty() {
            return Err(ForgeError::Workflow("name is required".into()));
        }
        if document.jobs.is_empty() {
            return Err(ForgeError::Workflow("at least one job is required".into()));
        }
        for (job_id, job) in &document.jobs {
            if job.steps.is_empty() {
                return Err(ForgeError::Workflow(format!(
                    "job '{job_id}' must define at least one step"
                )));
            }
            for need in &job.needs {
                if !document.jobs.contains_key(need) {
                    return Err(ForgeError::Workflow(format!(
                        "job '{job_id}' depends on unknown job '{need}'"
                    )));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::WorkflowParser;
    use crate::graph::WorkflowGraph;

    #[test]
    fn rejects_unknown_dependency() {
        let yaml = r#"
version: "1"
name: bad
jobs:
  build:
    name: Build
    needs: [missing]
    steps:
      - name: s
        run: echo
"#;
        let doc = WorkflowParser::parse_str(yaml).unwrap();
        assert!(WorkflowValidator::validate(&doc).is_err());
    }

    #[test]
    fn rejects_cycle() {
        let yaml = r#"
version: "1"
name: cycle
jobs:
  a:
    name: A
    needs: [b]
    steps:
      - name: s
        run: echo
  b:
    name: B
    needs: [a]
    steps:
      - name: s
        run: echo
"#;
        let doc = WorkflowParser::parse_str(yaml).unwrap();
        assert!(WorkflowValidator::validate(&doc).is_ok());
        assert!(WorkflowGraph::from_document(&doc).is_err());
    }
}