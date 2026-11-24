import { PageHeader } from "@/components/dashboard/page-header";
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
import { forgeGet, type ForgeProject } from "@/lib/forge-api";

export const metadata = { title: "Projects" };

export default async function ProjectsPage() {
  const projects = await forgeGet<ForgeProject[]>("projects");
  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        title="Projects"
        description="Connect repositories, set default branches, and scope workflows."
        actions={<Button>New project</Button>}
      />
      <Card>
        <CardHeader>
          <CardTitle>All projects</CardTitle>
          <CardDescription>
            Each project owns workflows, secrets, and integration settings.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Repository</TableHead>
                <TableHead>Branch</TableHead>
                <TableHead className="text-right">Workflows</TableHead>
                <TableHead>Last run</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {(projects ?? []).map((project) => (
                <TableRow key={project.id}>
                  <TableCell className="font-medium">{project.name}</TableCell>
                  <TableCell className="font-mono text-xs text-muted-foreground">
                    {project.repository}
                  </TableCell>
                  <TableCell>{project.default_branch}</TableCell>
                  <TableCell className="text-right tabular-nums">—</TableCell>
                  <TableCell className="text-muted-foreground text-sm">—</TableCell>
                </TableRow>
              ))}
              {projects?.length === 0 && <TableRow><TableCell colSpan={5} className="text-muted-foreground">No projects yet.</TableCell></TableRow>}
              {projects === null && <TableRow><TableCell colSpan={5} className="text-muted-foreground">Connect the dashboard with FORGE_API_URL and FORGE_API_TOKEN to load projects.</TableCell></TableRow>}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </div>
  );
}
