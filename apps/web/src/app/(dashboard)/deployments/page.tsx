import { PageHeader } from "@/components/dashboard/page-header";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export const metadata = { title: "Deployments" };

export default function DeploymentsPage() {
  return (
    <div className="flex flex-col gap-8">
      <PageHeader title="Deployments" description="Environment releases and rollback history." />
      <Card>
        <CardHeader>
          <CardTitle>Environments</CardTitle>
          <CardDescription>Track production, staging, and preview targets.</CardDescription>
        </CardHeader>
        <CardContent />
      </Card>
    </div>
  );
}