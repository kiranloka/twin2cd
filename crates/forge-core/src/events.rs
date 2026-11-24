//! Domain events for workflow lifecycle.

use crate::run::{JobStatus, RunStatus, StepStatus};
use forge_common::{JobId, RunId};

#[derive(Debug, Clone)]
pub enum WorkflowEvent {
    RunCreated {
        run_id: RunId,
    },
    RunStarted {
        run_id: RunId,
    },
    RunFinished {
        run_id: RunId,
        status: RunStatus,
    },
    JobStatusChanged {
        run_id: RunId,
        job_id: JobId,
        status: JobStatus,
    },
    StepStatusChanged {
        run_id: RunId,
        job_id: JobId,
        step_index: u32,
        status: StepStatus,
    },
}
