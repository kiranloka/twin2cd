import { PageHeader } from "@/components/dashboard/page-header";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export const metadata = { title: "Actions" };

const builtIns = [
  "forge/git-checkout@1",
  "forge/shell@1",
  "forge/docker-build@1",
  "forge/http@1",
];

export default function ActionsPage() {
  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        title="Actions"
        description="Built-in and installed actions available to workflow steps."
      />
      <Card>
        <CardHeader>
          <CardTitle>Installed</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-wrap gap-2">
          {builtIns.map((action) => (
            <Badge key={action} variant="outline" className="font-mono font-normal">
              {action}
            </Badge>
          ))}
        </CardContent>
      </Card>
    </div>
  );
}