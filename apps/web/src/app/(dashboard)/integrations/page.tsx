import { PageHeader } from "@/components/dashboard/page-header";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { mockGitCheckout } from "@/lib/mock-data";
import { Badge } from "@/components/ui/badge";
import { forgeGet } from "@/lib/forge-api";

export const metadata = { title: "Integrations" };

type WebhookDelivery = { delivery_id: string; event: string; received_at: string };

export default async function IntegrationsPage() {
  const deliveries = await forgeGet<WebhookDelivery[]>("webhooks/deliveries");
  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        title="Integrations"
        description="GitHub webhooks trigger workflows. Git checkout defaults apply to forge/git-checkout actions."
        actions={<Button>Connect GitHub</Button>}
      />

      <Tabs defaultValue="webhooks">
        <TabsList>
          <TabsTrigger value="webhooks">GitHub webhooks</TabsTrigger>
          <TabsTrigger value="git-checkout">Git checkout</TabsTrigger>
        </TabsList>

        <TabsContent value="webhooks" className="mt-6 flex flex-col gap-6">
          <Card id="webhooks">
            <CardHeader>
              <CardTitle>Webhook endpoints</CardTitle>
              <CardDescription>
                Signed GitHub deliveries are deduplicated and push events queue matching project workflows.
              </CardDescription>
            </CardHeader>
            <CardContent className="flex flex-col gap-6">
              <div className="grid gap-4 rounded-lg border border-border p-4 md:grid-cols-2">
                <div className="flex flex-col gap-2">
                  <Label htmlFor="hook-url">Payload URL</Label>
                  <Input
                    id="hook-url"
                    readOnly
                    value="https://forge.example.com/hooks/github"
                  />
                </div>
                <div className="flex flex-col gap-2">
                  <Label htmlFor="hook-secret">Webhook secret</Label>
                  <Input
                    id="hook-secret"
                    type="password"
                    placeholder="••••••••••••"
                    readOnly
                  />
                </div>
              </div>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Event</TableHead>
                    <TableHead>Delivery ID</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Received</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {(deliveries ?? []).map((delivery) => (
                    <TableRow key={delivery.delivery_id}>
                      <TableCell><Badge variant="secondary" className="font-normal">{delivery.event}</Badge></TableCell>
                      <TableCell className="font-mono text-xs">{delivery.delivery_id}</TableCell>
                      <TableCell>
                        <Badge variant="default">Accepted</Badge>
                      </TableCell>
                      <TableCell className="text-sm text-muted-foreground">
                        {new Date(Number(delivery.received_at) * 1000).toLocaleString()}
                      </TableCell>
                    </TableRow>
                  ))}
                  {deliveries?.length === 0 && <TableRow><TableCell colSpan={4} className="text-muted-foreground">No GitHub deliveries received.</TableCell></TableRow>}
                </TableBody>
              </Table>
              {deliveries === null && <p className="text-sm text-muted-foreground">Connect the dashboard with FORGE_API_URL and FORGE_API_TOKEN to load deliveries.</p>}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="git-checkout" className="mt-6 flex flex-col gap-6">
          <Card id="git-checkout">
            <CardHeader>
              <CardTitle>Git checkout defaults</CardTitle>
              <CardDescription>
                Used by <code className="text-xs">forge/git-checkout</code> steps.
                Shallow clones reduce data transfer on self-hosted runners.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Project</TableHead>
                    <TableHead>Ref</TableHead>
                    <TableHead>Depth</TableHead>
                    <TableHead>Submodules</TableHead>
                    <TableHead>LFS</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {mockGitCheckout.map((row) => (
                    <TableRow key={row.project}>
                      <TableCell className="font-medium">{row.project}</TableCell>
                      <TableCell className="font-mono text-xs">{row.default_ref}</TableCell>
                      <TableCell className="tabular-nums">
                        {row.fetch_depth === 0 ? "full" : row.fetch_depth}
                      </TableCell>
                      <TableCell>
                        <Switch checked={row.submodules} disabled aria-label="Submodules" />
                      </TableCell>
                      <TableCell>
                        <Switch checked={row.lfs} disabled aria-label="Git LFS" />
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
