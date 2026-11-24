use forge_common::JobId;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct JobLease {
    pub job_id: JobId,
    pub expires_at: Instant,
}

impl JobLease {
    pub fn new(job_id: JobId, ttl: Duration) -> Self {
        Self {
            job_id,
            expires_at: Instant::now() + ttl,
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}