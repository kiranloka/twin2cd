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
import { mockSecrets } from "@/lib/mock-data";

export const metadata = { title: "Secrets" };

export default function SecretsPage() {
  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        title="Secrets"
        description="Encrypted values injected at run time. Never stored in Forgefile YAML."
        actions={<Button>Add secret</Button>}
      />
      <Card>
        <CardHeader>
          <CardTitle>Organization & project secrets</CardTitle>
          <CardDescription>
            Values are masked in logs before persistence. Runners receive only
            secrets for their assigned job.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Scope</TableHead>
                <TableHead>Project</TableHead>
                <TableHead>Updated</TableHead>
                <TableHead>Last used</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {mockSecrets.map((secret) => (
                <TableRow key={secret.id}>
                  <TableCell className="font-mono text-sm">{secret.name}</TableCell>
                  <TableCell className="capitalize">{secret.scope}</TableCell>
                  <TableCell className="text-muted-foreground">
                    {secret.project ?? "—"}
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {new Date(secret.updated_at).toLocaleDateString()}
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {secret.last_used_at
                      ? new Date(secret.last_used_at).toLocaleString()
                      : "Never"}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </div>
  );
}