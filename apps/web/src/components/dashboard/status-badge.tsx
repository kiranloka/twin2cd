import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

const styles: Record<string, string> = {
  success: "bg-emerald-500/10 text-emerald-700 dark:text-emerald-400",
  failed: "bg-destructive/10 text-destructive",
  running: "bg-primary/10 text-primary",
  online: "bg-emerald-500/10 text-emerald-700 dark:text-emerald-400",
  offline: "bg-muted text-muted-foreground",
  draining: "bg-amber-500/10 text-amber-800 dark:text-amber-400",
};

export function StatusBadge({
  status,
  className,
}: {
  status: string;
  className?: string;
}) {
  const key = status.toLowerCase();
  return (
    <Badge
      variant="secondary"
      className={cn("capitalize font-normal", styles[key], className)}
    >
      {status}
    </Badge>
  );
}