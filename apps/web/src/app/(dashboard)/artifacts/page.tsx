import { PageHeader } from "@/components/dashboard/page-header";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export const metadata = { title: "Artifacts" };

export default function ArtifactsPage() {
  return (
    <div className="flex flex-col gap-8">
      <PageHeader title="Artifacts" description="Build outputs and cached layers." />
      <Card>
        <CardHeader>
          <CardTitle>No artifacts yet</CardTitle>
          <CardDescription>Artifact registry hooks into forge-artifacts.</CardDescription>
        </CardHeader>
        <CardContent />
      </Card>
    </div>
  );
}