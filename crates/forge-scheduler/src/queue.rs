use forge_common::JobId;
use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct JobQueue {
    pending: VecDeque<JobId>,
}

impl JobQueue {
    pub fn enqueue(&mut self, job_id: JobId) {
        self.pending.push_back(job_id);
    }

    pub fn dequeue(&mut self) -> Option<JobId> {
        self.pending.pop_front()
    }
}