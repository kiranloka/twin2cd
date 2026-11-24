import { PageHeader } from "@/components/dashboard/page-header";
import { StatusBadge } from "@/components/dashboard/status-badge";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
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
import { forgeGet, type ForgeRunner } from "@/lib/forge-api";
import { Info } from "lucide-react";

export const metadata = { title: "Runners" };

const milestoneThreeChecklist = [
  { item: "Runner registration API + scoped tokens", done: true },
  { item: "gRPC outbound control stream", done: true },
  { item: "Heartbeat stream", done: true },
  { item: "Job assignment + database leases", done: true },
  { item: "Idempotent completion reports", done: true },
  { item: "Expired-lease recovery", done: true },
];

export default async function RunnersPage() {
  const runners = await forgeGet<ForgeRunner[]>("runners");
  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        title="Runners"
        description="Distributed execution workers. The controller schedules; runners never expose inbound ports."
        actions={<Button variant="outline">Register runner</Button>}
      />

      <Alert>
        <Info className="size-4" />
        <AlertTitle>Milestone 3 status</AlertTitle>
        <AlertDescription className="flex flex-col gap-2">
          <span>
            Runner status, leases, and capacity are loaded from the Forge API.
          </span>
          <ul className="flex flex-col gap-1 text-sm">
            {milestoneThreeChecklist.map(({ item, done }) => (
              <li key={item} className="flex items-center gap-2">
                <Badge
                  variant={done === true ? "default" : "secondary"}
                  className="shrink-0 text-[10px] uppercase"
                >
                  {done ? "done" : "todo"}
                </Badge>
                {item}
              </li>
            ))}
          </ul>
        </AlertDescription>
      </Alert>

      <Card>
        <CardHeader>
          <CardTitle>Registered runners</CardTitle>
          <CardDescription>
            Labels and capacity drive the scheduler. Heartbeats expire after 15s
            (default).
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Labels</TableHead>
                <TableHead>Executors</TableHead>
                <TableHead className="text-right">Jobs</TableHead>
                <TableHead>Last heartbeat</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {(runners ?? []).map((runner) => (
                <TableRow key={runner.id}>
                  <TableCell>
                    <div className="flex flex-col gap-0.5">
                      <span className="font-medium">{runner.name}</span>
                      <span className="font-mono text-xs text-muted-foreground">
                        {runner.id}
                      </span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <StatusBadge status={runner.status} />
                  </TableCell>
                  <TableCell>
                    <div className="flex flex-wrap gap-1">
                      {(runner.labels ?? []).map((label) => (
                        <Badge key={label} variant="outline" className="font-normal">
                          {label}
                        </Badge>
                      ))}
                    </div>
                  </TableCell>
                  <TableCell className="text-muted-foreground text-sm">
                    {(runner.executors ?? []).join(", ")}
                  </TableCell>
                  <TableCell className="text-right tabular-nums">
                    {runner.active_jobs}/{runner.capacity?.max_concurrent_jobs ?? 0}
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {runner.last_heartbeat_at ? new Date(runner.last_heartbeat_at).toLocaleString() : "—"}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
          {runners === null && <p className="mt-4 text-sm text-muted-foreground">Connect the dashboard with FORGE_API_URL and FORGE_API_TOKEN to load runners.</p>}
        </CardContent>
      </Card>
    </div>
  );
}
