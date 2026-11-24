import { PageHeader } from "@/components/dashboard/page-header";
import { StatusBadge } from "@/components/dashboard/status-badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import Link from "next/link";
import { forgeGet, type ForgeRun } from "@/lib/forge-api";

export const metadata = { title: "Executions" };

export default async function ExecutionsPage() {
  const runs = await forgeGet<ForgeRun[]>("runs");
  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        title="Executions"
        description="Run history and live logs from connected runners."
      />
      <Card>
        <CardHeader>
          <CardTitle>All runs</CardTitle>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>ID</TableHead>
                <TableHead>Project</TableHead>
                <TableHead>Workflow</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Started</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {(runs ?? []).map((run) => (
                <TableRow key={run.run_id}>
                  <TableCell className="font-mono text-xs"><Link className="hover:underline" href={`/executions/${encodeURIComponent(run.run_id)}`}>{run.run_id}</Link></TableCell>
                  <TableCell>—</TableCell>
                  <TableCell>{run.workflow_name}</TableCell>
                  <TableCell>
                    <StatusBadge status={run.status as "queued" | "running" | "completed" | "failed" | "cancelled"} />
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {new Date(run.created_at * 1000).toLocaleString()}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
          {runs === null && <p className="mt-4 text-sm text-muted-foreground">Connect the dashboard with FORGE_API_URL and FORGE_API_TOKEN to load runs.</p>}
        </CardContent>
      </Card>
    </div>
  );
}
