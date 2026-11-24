"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";
import { mainNav, resourcesNav, settingsNav } from "@/lib/navigation";
import { cn } from "@/lib/utils";
import { AudioWaveform } from "lucide-react";

function NavGroup({
  label,
  items,
}: {
  label: string;
  items: typeof mainNav;
}) {
  const pathname = usePathname();

  return (
    <SidebarGroup>
      <SidebarGroupLabel>{label}</SidebarGroupLabel>
      <SidebarGroupContent>
        <SidebarMenu>
          {items.map((item) => {
            const active =
              item.href === "/"
                ? pathname === "/"
                : pathname.startsWith(item.href);
            const Icon = item.icon;
            return (
              <SidebarMenuItem key={item.href}>
                <SidebarMenuButton
                  render={<Link href={item.href} />}
                  isActive={active}
                  className={cn(
                    "h-9 font-medium text-[13px]",
                    active && "bg-foreground text-background hover:bg-foreground/90 hover:text-background",
                  )}
                >
                  <Icon />
                  <span>{item.title}</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            );
          })}
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  );
}

export function AppSidebar() {
  return (
    <Sidebar collapsible="none" variant="sidebar" className="border-r border-sidebar-border">
      <SidebarHeader className="h-[62px] justify-center border-b border-sidebar-border px-4">
        <div className="flex items-center gap-2.5">
          <span className="grid size-8 place-items-center rounded-lg bg-foreground text-background">
            <AudioWaveform className="size-4" />
          </span>
          <div className="min-w-0">
            <p className="text-sm font-semibold leading-tight tracking-tight">Forge</p>
            <p className="text-[11px] leading-tight text-muted-foreground">Workflow platform</p>
          </div>
        </div>
      </SidebarHeader>
      <SidebarContent>
        <div className="pt-3" />
        <NavGroup label="Workspace" items={mainNav} />
        <NavGroup label="Resources" items={resourcesNav} />
        <NavGroup label="Settings" items={settingsNav} />
      </SidebarContent>
      <SidebarFooter className="border-t border-sidebar-border p-4">
        <div className="flex items-center justify-between text-xs">
          <span className="text-muted-foreground">Workspace</span>
          <span className="rounded-md border border-border px-1.5 py-0.5 font-medium text-foreground">Free</span>
        </div>
        <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-muted">
          <div className="h-full w-[38%] rounded-full bg-foreground" />
        </div>
        <p className="mt-2 text-[11px] text-muted-foreground">38% of monthly run minutes used</p>
      </SidebarFooter>
    </Sidebar>
  );
}
