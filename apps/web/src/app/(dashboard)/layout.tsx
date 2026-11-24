import { AppSidebar } from "@/components/dashboard/app-sidebar";
import { SidebarInset, SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar";
import { Separator } from "@/components/ui/separator";
import { Input } from "@/components/ui/input";
import { Search } from "lucide-react";

export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <SidebarProvider>
      <AppSidebar />
      <SidebarInset>
        <header className="sticky top-0 z-20 flex h-[62px] shrink-0 items-center gap-3 border-b border-border/80 bg-background/95 px-4 backdrop-blur md:px-6">
          <SidebarTrigger className="-ml-1 text-muted-foreground" />
          <Separator orientation="vertical" className="h-5" />
          <div className="relative w-full max-w-sm">
            <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input aria-label="Search Forge" placeholder="Search projects, workflows..." className="h-9 border-border/80 bg-white pl-9 text-sm shadow-none placeholder:text-muted-foreground/80" />
          </div>
          <div className="ml-auto hidden items-center gap-2 text-xs text-muted-foreground sm:flex">
            <span className="size-2 rounded-full bg-emerald-500" />
            Control plane online
          </div>
        </header>
        <main className="flex flex-1 flex-col px-4 py-7 md:px-7 lg:px-10">
          <div className="mx-auto flex w-full max-w-[1368px] flex-1 flex-col gap-7">
            {children}
          </div>
        </main>
      </SidebarInset>
    </SidebarProvider>
  );
}
