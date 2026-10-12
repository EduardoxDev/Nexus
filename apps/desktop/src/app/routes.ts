import {
  BookOpenCheck,
  Cable,
  FolderGit2,
  HeartPulse,
  LayoutDashboard,
  PackageOpen,
  Radar,
  ScanSearch,
  SlidersHorizontal,
  type LucideIcon,
} from "lucide-react";

import { t } from "@/lib/i18n/pt-BR";

export type RouteId =
  | "overview"
  | "projects"
  | "search"
  | "contextPacks"
  | "memory"
  | "freshness"
  | "integrations"
  | "settings"
  | "diagnostics";

export interface RouteDef {
  id: RouteId;
  label: string;
  description: string;
  icon: LucideIcon;
  group: "workspace" | "system";
  /** Fase do roadmap em que a seção passa a existir; `null` = disponível. */
  plannedPhase: number | null;
}

export const ROUTES: readonly RouteDef[] = [
  {
    id: "overview",
    icon: LayoutDashboard,
    group: "workspace",
    plannedPhase: null,
    ...t.nav.overview,
  },
  { id: "projects", icon: FolderGit2, group: "workspace", plannedPhase: null, ...t.nav.projects },
  { id: "search", icon: ScanSearch, group: "workspace", plannedPhase: null, ...t.nav.search },
  {
    id: "contextPacks",
    icon: PackageOpen,
    group: "workspace",
    plannedPhase: null,
    ...t.nav.contextPacks,
  },
  { id: "memory", icon: BookOpenCheck, group: "workspace", plannedPhase: null, ...t.nav.memory },
  { id: "freshness", icon: Radar, group: "workspace", plannedPhase: null, ...t.nav.freshness },
  {
    id: "integrations",
    icon: Cable,
    group: "workspace",
    plannedPhase: null,
    ...t.nav.integrations,
  },
  {
    id: "settings",
    icon: SlidersHorizontal,
    group: "system",
    plannedPhase: null,
    ...t.nav.settings,
  },
  {
    id: "diagnostics",
    icon: HeartPulse,
    group: "system",
    plannedPhase: null,
    ...t.nav.diagnostics,
  },
];

export function routeDef(id: RouteId): RouteDef {
  const def = ROUTES.find((r) => r.id === id);
  if (!def) throw new Error(`rota desconhecida: ${id}`);
  return def;
}

export function isAvailable(id: RouteId): boolean {
  return routeDef(id).plannedPhase === null;
}
