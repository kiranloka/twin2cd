import { PageHeader } from "@/components/dashboard/page-header";
import { WorkflowEditor } from "./workflow-editor";

export const metadata = { title: "Workflows" };

export default function WorkflowsPage() {
  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        title="Workflows"
        description="Edit Forgefile YAML, validate it with the controller, and inspect job dependencies."
      />
      <WorkflowEditor />
    </div>
  );
}
