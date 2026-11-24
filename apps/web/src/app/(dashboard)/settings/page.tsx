import { PageHeader } from "@/components/dashboard/page-header";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

export const metadata = { title: "Settings" };

export default function SettingsPage() {
  return (
    <div className="flex flex-col gap-8">
      <PageHeader title="Settings" description="Organization profile and API access." />
      <Card>
        <CardHeader>
          <CardTitle>General</CardTitle>
          <CardDescription>Self-hosted control plane defaults.</CardDescription>
        </CardHeader>
        <CardContent className="flex max-w-md flex-col gap-4">
          <div className="flex flex-col gap-2">
            <Label htmlFor="org">Organization</Label>
            <Input id="org" defaultValue="acme" />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="api">Controller URL</Label>
            <Input id="api" defaultValue="http://localhost:8080" readOnly />
          </div>
        </CardContent>
      </Card>
    </div>
  );
}