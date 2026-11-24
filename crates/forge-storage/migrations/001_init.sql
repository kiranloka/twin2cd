-- Portable schema for SQLite (default) and PostgreSQL.

CREATE TABLE IF NOT EXISTS organizations (
    id TEXT PRIMARY KEY NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY NOT NULL,
    email TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS memberships (
    user_id TEXT NOT NULL,
    organization_id TEXT NOT NULL,
    role TEXT NOT NULL,
    PRIMARY KEY (user_id, organization_id),
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (organization_id) REFERENCES organizations(id)
);

CREATE TABLE IF NOT EXISTS api_tokens (
    id TEXT PRIMARY KEY NOT NULL,
    organization_id TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    role TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT,
    FOREIGN KEY (organization_id) REFERENCES organizations(id)
);

CREATE TABLE IF NOT EXISTS runners (
    id TEXT PRIMARY KEY NOT NULL,
    organization_id TEXT NOT NULL,
    name TEXT NOT NULL,
    platform TEXT NOT NULL,
    architecture TEXT NOT NULL,
    labels_json TEXT NOT NULL,
    executors_json TEXT NOT NULL,
    status TEXT NOT NULL,
    last_heartbeat_at TEXT,
    FOREIGN KEY (organization_id) REFERENCES organizations(id)
);

CREATE TABLE IF NOT EXISTS audit_logs (
    id TEXT PRIMARY KEY NOT NULL,
    organization_id TEXT NOT NULL,
    actor_id TEXT,
    action TEXT NOT NULL,
    resource TEXT NOT NULL,
    metadata_json TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS billing_accounts (
    organization_id TEXT PRIMARY KEY NOT NULL,
    plan TEXT NOT NULL DEFAULT 'free',
    stripe_customer_id TEXT,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (organization_id) REFERENCES organizations(id)
);

CREATE INDEX IF NOT EXISTS idx_memberships_org ON memberships (organization_id);
CREATE INDEX IF NOT EXISTS idx_runners_org ON runners (organization_id);
CREATE INDEX IF NOT EXISTS idx_audit_org_created ON audit_logs (organization_id, created_at);
