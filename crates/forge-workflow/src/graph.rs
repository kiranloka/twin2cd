use forge_core::{ForgeError, ForgeResult, WorkflowDocument};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
pub struct WorkflowGraph {
    pub order: Vec<String>,
}

impl WorkflowGraph {
    pub fn from_document(document: &WorkflowDocument) -> ForgeResult<Self> {
        let mut in_degree: HashMap<&str, usize> =
            document.jobs.keys().map(|k| (k.as_str(), 0)).collect();
        let mut edges: HashMap<&str, Vec<&str>> = HashMap::new();

        for (job_id, job) in &document.jobs {
            for need in &job.needs {
                edges.entry(need.as_str()).or_default().push(job_id.as_str());
                if let Some(deg) = in_degree.get_mut(job_id.as_str()) {
                    *deg += 1;
                }
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut order = Vec::with_capacity(document.jobs.len());
        let mut visited = 0usize;

        while let Some(node) = queue.pop_front() {
            order.push(node.to_string());
            visited += 1;
            if let Some(children) = edges.get(node) {
                for child in children {
                    let deg = in_degree.get_mut(child).expect("child in graph");
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(child);
                    }
                }
            }
        }

        if visited != document.jobs.len() {
            return Err(ForgeError::Workflow(
                "workflow contains cyclic job dependencies".into(),
            ));
        }

        Ok(Self { order })
    }
}