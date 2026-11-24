import Link from "next/link";
import { PageHeader } from "@/components/dashboard/page-header";
import { StatusBadge } from "@/components/dashboard/status-badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { forgeGet, type ForgeProject, type ForgeRun, type ForgeRunner } from "@/lib/forge-api";

export default async function OverviewPage() {
  const [projects, runners, runs] = await Promise.all([
    forgeGet<ForgeProject[]>("projects"), forgeGet<ForgeRunner[]>("runners"), forgeGet<ForgeRun[]>("runs"),
  ]);
  const onlineRunners = runners?.filter((r) => r.status === "online").length ?? 0;

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        title="Overview"
        description="System health, recent workflow runs, and runner availability."
        actions={
          <Button render={<Link href="/executions" />}>View executions</Button>
        }
      />

      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <Card>
          <CardHeader className="gap-3 pb-1">
            <CardDescription className="text-[13px]">Projects</CardDescription>
            <CardTitle className="text-[28px] font-semibold tabular-nums tracking-tight">
              {projects?.length ?? "—"}
            </CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="gap-3 pb-1">
            <CardDescription className="text-[13px]">Runners online</CardDescription>
            <CardTitle className="text-[28px] font-semibold tabular-nums tracking-tight">
              {onlineRunners}/{runners?.length ?? "—"}
            </CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="gap-3 pb-1">
            <CardDescription className="text-[13px]">Runs (24h)</CardDescription>
            <CardTitle className="text-[28px] font-semibold tabular-nums tracking-tight">{runs?.length ?? "—"}</CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="gap-3 pb-1">
            <CardDescription className="text-[13px]">Failed runs</CardDescription>
            <CardTitle className="text-[28px] font-semibold tabular-nums tracking-tight text-destructive">{runs?.filter((run) => run.status === "failed").length ?? "—"}</CardTitle>
          </CardHeader>
        </Card>
      </div>

      <Card>
        <CardHeader className="border-b border-border/80 pb-4">
          <CardTitle>Recent executions</CardTitle>
          <CardDescription>Latest workflow runs across projects.</CardDescription>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Run</TableHead>
                <TableHead>Workflow</TableHead>
                <TableHead>Trigger</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className="text-right">Duration</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {(runs ?? []).slice(0, 10).map((run) => (
                <TableRow key={run.run_id}>
                  <TableCell className="font-mono text-xs"><Link className="hover:underline" href={`/executions/${encodeURIComponent(run.run_id)}`}>{run.run_id}</Link></TableCell>
                  <TableCell>{run.workflow_name}</TableCell>
                  <TableCell className="text-muted-foreground">{run.trigger}</TableCell>
                  <TableCell>
                    <StatusBadge status={run.status as "queued" | "running" | "completed" | "failed" | "cancelled"} />
                  </TableCell>
                  <TableCell className="text-right tabular-nums">
                    {new Date(run.created_at * 1000).toLocaleTimeString()}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
          {runs === null && <p className="mt-4 text-sm text-muted-foreground">Connect the dashboard with FORGE_API_URL and FORGE_API_TOKEN to load live data.</p>}
        </CardContent>
      </Card>
    </div>
  );
}
