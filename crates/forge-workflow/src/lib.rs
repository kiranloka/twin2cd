pub mod compiler;
pub mod expressions;
pub mod graph;
pub mod parser;
pub mod validator;

pub use compiler::{CompiledWorkflow, WorkflowCompiler};
pub use expressions::{contains_expression, evaluate_template, ExpressionContext};
pub use graph::WorkflowGraph;
pub use parser::WorkflowParser;
pub use validator::WorkflowValidator;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_job_dag_in_dependency_order() {
        let yaml = r#"
version: "1"
name: demo
jobs:
  build:
    name: Build
    needs: [test]
    steps:
      - name: Build step
        run: echo build
  test:
    name: Test
    steps:
      - name: Test step
        run: echo test
"#;
        let document = WorkflowParser::parse_str(yaml).expect("parse");
        let compiled = WorkflowCompiler::compile(document).expect("compile");
        assert_eq!(compiled.graph.order, vec!["test", "build"]);
    }
}