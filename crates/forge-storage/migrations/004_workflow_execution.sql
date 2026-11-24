CREATE TABLE IF NOT EXISTS workflow_runs (
    run_id TEXT PRIMARY KEY NOT NULL,
    organization_id TEXT NOT NULL,
    workflow_name TEXT NOT NULL,
    trigger TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (organization_id) REFERENCES organizations(id)
);

CREATE TABLE IF NOT EXISTS workflow_jobs (
    job_id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL,
    organization_id TEXT NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    runner_id TEXT,
    lease_expires_at INTEGER,
    idempotency_key TEXT,
    exit_code INTEGER,
    FOREIGN KEY (run_id) REFERENCES workflow_runs(run_id),
    FOREIGN KEY (organization_id) REFERENCES organizations(id)
);

CREATE TABLE IF NOT EXISTS run_logs (
    log_id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL,
    organization_id TEXT NOT NULL,
    stream TEXT NOT NULL,
    line TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (run_id) REFERENCES workflow_runs(run_id),
    FOREIGN KEY (organization_id) REFERENCES organizations(id)
);

CREATE INDEX IF NOT EXISTS idx_runs_org_created
    ON workflow_runs (organization_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_jobs_claim
    ON workflow_jobs (organization_id, status, created_at);
CREATE INDEX IF NOT EXISTS idx_jobs_lease
    ON workflow_jobs (status, lease_expires_at);
CREATE INDEX IF NOT EXISTS idx_run_logs_org_run
    ON run_logs (organization_id, run_id, created_at);
