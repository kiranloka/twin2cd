import "server-only";

const apiBase = process.env.FORGE_API_URL;
const apiToken = process.env.FORGE_API_TOKEN;

export async function forgeGet<T>(path: string): Promise<T | null> {
  if (!apiBase || !apiToken) return null;
  try {
    const response = await fetch(`${apiBase.replace(/\/$/, "")}/api/v1/${path.replace(/^\//, "")}`, {
      headers: { authorization: `Bearer ${apiToken}`, accept: "application/json" },
      cache: "no-store",
    });
    if (!response.ok) return null;
    return (await response.json()) as T;
  } catch {
    return null;
  }
}

export type ForgeProject = { id: string; name: string; slug: string; repository: string; default_branch: string };
export type ForgeRunner = {
  id: string; name: string; status: string; labels: string[]; executors: string[];
  capacity: { max_concurrent_jobs: number }; active_jobs: number; last_heartbeat_at: string | null;
};
export type ForgeRun = {
  run_id: string; workflow_name: string; trigger: string; status: string; created_at: number;
};
export type ForgeLog = { log_id: string; stream: string; line: string; created_at: number };
