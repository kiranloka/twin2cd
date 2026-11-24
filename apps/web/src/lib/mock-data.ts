import type {
  GitCheckoutDefaults,
  GitHubWebhookRecord,
  ProjectRecord,
  RunnerRecord,
  SecretRecord,
} from "@forge/types";

export const mockProjects: ProjectRecord[] = [
  {
    id: "prj_onepos",
    name: "OnePOS",
    slug: "onepos",
    repository: "github.com/acme/onepos",
    default_branch: "main",
    last_run_at: "2026-11-23T16:40:00Z",
    workflow_count: 3,
  },
  {
    id: "prj_forge",
    name: "Forge",
    slug: "forge",
    repository: "github.com/acme/forge",
    default_branch: "main",
    last_run_at: null,
    workflow_count: 1,
  },
];

export const mockRunners: RunnerRecord[] = [
  {
    id: "runner-001",
    name: "production-deployer",
    status: "online",
    platform: "linux",
    architecture: "x86_64",
    labels: ["linux", "docker", "deployment"],
    executors: ["native", "container"],
    capacity: { cpu: 2, memory_mb: 1024, max_concurrent_jobs: 1 },
    active_jobs: 0,
    last_heartbeat_at: "2026-11-23T17:10:12Z",
  },
  {
    id: "runner-002",
    name: "ci-builder",
    status: "offline",
    platform: "linux",
    architecture: "aarch64",
    labels: ["linux", "build"],
    executors: ["container"],
    capacity: { cpu: 4, memory_mb: 4096, max_concurrent_jobs: 2 },
    active_jobs: 0,
    last_heartbeat_at: "2026-11-23T14:02:00Z",
  },
];

export const mockSecrets: SecretRecord[] = [
  {
    id: "sec_1",
    name: "DEPLOY_HOST",
    scope: "project",
    project: "onepos",
    updated_at: "2026-11-20T10:00:00Z",
    last_used_at: "2026-11-23T16:40:00Z",
  },
  {
    id: "sec_2",
    name: "GITHUB_TOKEN",
    scope: "organization",
    updated_at: "2026-11-18T08:30:00Z",
    last_used_at: null,
  },
];

export const mockWebhooks: GitHubWebhookRecord[] = [
  {
    id: "wh_1",
    project: "onepos",
    delivery_url: "https://forge.example.com/hooks/github/onepos",
    events: ["push", "pull_request"],
    active: true,
    last_delivery_at: "2026-11-23T15:22:00Z",
  },
];

export const mockGitCheckout: GitCheckoutDefaults[] = [
  {
    project: "onepos",
    fetch_depth: 1,
    submodules: false,
    lfs: false,
    default_ref: "${{ git.ref }}",
  },
  {
    project: "forge",
    fetch_depth: 0,
    submodules: true,
    lfs: false,
    default_ref: "main",
  },
];

export const mockExecutions = [
  {
    id: "run_a1",
    workflow: "onepos-production",
    project: "onepos",
    status: "success" as const,
    triggered_by: "git.push",
    started_at: "2026-11-23T16:35:00Z",
    duration_sec: 312,
  },
  {
    id: "run_b2",
    workflow: "forge-ci",
    project: "forge",
    status: "failed" as const,
    triggered_by: "manual",
    started_at: "2026-11-22T11:00:00Z",
    duration_sec: 48,
  },
];