import { notFound } from "next/navigation";
import { PageHeader } from "@/components/dashboard/page-header";
import { StatusBadge } from "@/components/dashboard/status-badge";
import { forgeGet, type ForgeRun } from "@/lib/forge-api";
import { LiveLogs } from "./live-logs";

export default async function RunPage({ params }: { params: Promise<{ runId: string }> }) {
  const { runId } = await params;
  const runs = await forgeGet<ForgeRun[]>("runs");
  const run = runs?.find((item) => item.run_id === runId);
  if (!run) notFound();
  return <div className="flex flex-col gap-6">
    <PageHeader title={run.workflow_name} description={`${run.run_id} · ${run.trigger} · ${new Date(run.created_at * 1000).toLocaleString()}`} />
    <div><StatusBadge status={run.status} /></div>
    <LiveLogs runId={run.run_id} />
  </div>;
}
