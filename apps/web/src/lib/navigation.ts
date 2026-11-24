import type { LucideIcon } from "lucide-react";
import {
  Box,
  FolderGit2,
  GitBranch,
  History,
  KeyRound,
  LayoutDashboard,
  Package,
  Plug,
  Rocket,
  Server,
  Settings,
  Workflow,
} from "lucide-react";

export type NavItem = {
  title: string;
  href: string;
  icon: LucideIcon;
  description?: string;
};

export const mainNav: NavItem[] = [
  {
    title: "Overview",
    href: "/",
    icon: LayoutDashboard,
    description: "Status and recent activity",
  },
  {
    title: "Projects",
    href: "/projects",
    icon: FolderGit2,
    description: "Repositories and teams",
  },
  {
    title: "Workflows",
    href: "/workflows",
    icon: Workflow,
    description: "Forgefile definitions",
  },
  {
    title: "Executions",
    href: "/executions",
    icon: History,
    description: "Runs and live logs",
  },
  {
    title: "Runners",
    href: "/runners",
    icon: Server,
    description: "Workers and capacity",
  },
];

export const resourcesNav: NavItem[] = [
  { title: "Artifacts", href: "/artifacts", icon: Package },
  { title: "Deployments", href: "/deployments", icon: Rocket },
  { title: "Secrets", href: "/secrets", icon: KeyRound },
  { title: "Actions", href: "/actions", icon: Box },
];

export const settingsNav: NavItem[] = [
  {
    title: "Integrations",
    href: "/integrations",
    icon: Plug,
    description: "GitHub webhooks and git checkout",
  },
  {
    title: "Settings",
    href: "/settings",
    icon: Settings,
  },
];

export const integrationsSubNav = [
  { title: "GitHub webhooks", href: "/integrations#webhooks", icon: GitBranch },
  {
    title: "Git checkout",
    href: "/integrations#git-checkout",
    icon: FolderGit2,
  },
] as const;