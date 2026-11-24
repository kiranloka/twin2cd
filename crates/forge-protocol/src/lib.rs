//! Controller ↔ runner control messages (JSON today; gRPC/protobuf in Milestone 3).

use forge_common::{JobId, RunnerId};
use serde::{Deserialize, Serialize};

pub mod v1 {
    tonic::include_proto!("forge.protocol.v1");
}

pub const DEFAULT_HEARTBEAT_INTERVAL_SECS: u64 = 15;
pub const DEFAULT_JOB_LEASE_TTL_SECS: u64 = 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerRegistration {
    pub runner_id: RunnerId,
    pub name: String,
    pub platform: String,
    pub architecture: String,
    pub labels: Vec<String>,
    pub executors: Vec<String>,
    pub capacity: RunnerCapacity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerCapacity {
    pub cpu: u32,
    pub memory_mb: u32,
    pub max_concurrent_jobs: u32,
}

impl Default for RunnerCapacity {
    fn default() -> Self {
        Self {
            cpu: 1,
            memory_mb: 1024,
            max_concurrent_jobs: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerHeartbeat {
    pub runner_id: RunnerId,
    pub sent_at: String,
    pub active_jobs: u32,
    pub status: ConnectionStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    Idle,
    Busy,
    Draining,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobAssignment {
    pub job_id: JobId,
    pub run_id: String,
    pub lease_ttl_secs: u64,
    pub idempotency_key: String,
    pub name: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobLeaseRenewal {
    pub job_id: JobId,
    pub runner_id: RunnerId,
    pub extend_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCompletionReport {
    pub job_id: JobId,
    pub runner_id: RunnerId,
    pub exit_code: i32,
    pub idempotency_key: String,
}
