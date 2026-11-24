use forge_protocol::{ConnectionStatus, RunnerHeartbeat, DEFAULT_HEARTBEAT_INTERVAL_SECS};
use forge_common::RunnerId;
use tracing::trace;

pub fn interval_secs() -> u64 {
    DEFAULT_HEARTBEAT_INTERVAL_SECS
}

pub fn send(runner_id: &RunnerId, active_jobs: u32) {
    let _payload = RunnerHeartbeat {
        runner_id: runner_id.clone(),
        sent_at: chrono_lite_now(),
        active_jobs,
        status: if active_jobs > 0 {
            ConnectionStatus::Busy
        } else {
            ConnectionStatus::Idle
        },
    };
    trace!(?runner_id, "heartbeat sent");
}

fn chrono_lite_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{ms}")
}