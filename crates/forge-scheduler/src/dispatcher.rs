use crate::queue::JobQueue;
use forge_common::{JobId, RunnerId};

#[derive(Debug, Default)]
pub struct JobDispatcher {
    queue: JobQueue,
}

impl JobDispatcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn schedule(&mut self, job_id: JobId) {
        self.queue.enqueue(job_id);
    }

    pub fn assign_next(&mut self, _runner_id: &RunnerId) -> Option<JobId> {
        self.queue.dequeue()
    }
}