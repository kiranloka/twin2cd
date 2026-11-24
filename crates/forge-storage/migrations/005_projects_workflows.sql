CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY NOT NULL,
    organization_id TEXT NOT NULL,
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    repository TEXT NOT NULL,
    default_branch TEXT NOT NULL DEFAULT 'main',
    created_at INTEGER NOT NULL,
    UNIQUE (organization_id, slug),
    FOREIGN KEY (organization_id) REFERENCES organizations(id)
);

CREATE TABLE IF NOT EXISTS workflow_definitions (
    workflow_id TEXT PRIMARY KEY NOT NULL,
    organization_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    yaml TEXT NOT NULL,
    version INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (organization_id, project_id, name),
    FOREIGN KEY (organization_id) REFERENCES organizations(id),
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

ALTER TABLE workflow_runs ADD COLUMN project_id TEXT;

CREATE INDEX IF NOT EXISTS idx_projects_org_name ON projects (organization_id, name);
CREATE INDEX IF NOT EXISTS idx_workflows_project ON workflow_definitions (organization_id, project_id);
