export type ForgeWorkflowVersion = "1";

export interface ValidateWorkflowResponse {
  valid: boolean;
  job_order: string[];
  error?: string;
}

export type RunnerStatus = "online" | "offline" | "draining";

export interface RunnerCapacity {
  cpu: number;
  memory_mb: number;
  max_concurrent_jobs: number;
}

export interface RunnerRecord {
  id: string;
  name: string;
  status: RunnerStatus;
  platform: string;
  architecture: string;
  labels: string[];
  executors: ("native" | "container" | "remote")[];
  capacity: RunnerCapacity;
  active_jobs: number;
  last_heartbeat_at: string;
}

export interface ProjectRecord {
  id: string;
  name: string;
  slug: string;
  repository: string;
  default_branch: string;
  last_run_at: string | null;
  workflow_count: number;
}

export interface SecretRecord {
  id: string;
  name: string;
  scope: "organization" | "project";
  project?: string;
  updated_at: string;
  last_used_at: string | null;
}

export interface GitHubWebhookRecord {
  id: string;
  project: string;
  delivery_url: string;
  events: string[];
  active: boolean;
  last_delivery_at: string | null;
}

export interface GitCheckoutDefaults {
  project: string;
  fetch_depth: number;
  submodules: boolean;
  lfs: boolean;
  default_ref: string;
}