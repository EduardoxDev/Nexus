import {
  Activity,
  Brain,
  FolderGit2,
  History,
  LayoutDashboard,
  Package,
  Plug,
  Search,
  Settings,
  type LucideIcon,
} from "lucide-react";

import { t } from "@/lib/i18n/pt-BR";

/** Última fase do roadmap entregue (as etapas até ela aparecem como disponíveis). */
export const CURRENT_PHASE = 3;

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
  { id: "search", icon: Search, group: "workspace", plannedPhase: null, ...t.nav.search },
  {
    id: "contextPacks",
    icon: Package,
    group: "workspace",
    plannedPhase: null,
    ...t.nav.contextPacks,
  },
  { id: "memory", icon: Brain, group: "workspace", plannedPhase: 4, ...t.nav.memory },
  { id: "freshness", icon: History, group: "workspace", plannedPhase: 4, ...t.nav.freshness },
  { id: "integrations", icon: Plug, group: "workspace", plannedPhase: 5, ...t.nav.integrations },
  { id: "settings", icon: Settings, group: "system", plannedPhase: null, ...t.nav.settings },
  { id: "diagnostics", icon: Activity, group: "system", plannedPhase: null, ...t.nav.diagnostics },
];

export function routeDef(id: RouteId): RouteDef {
  const def = ROUTES.find((r) => r.id === id);
  if (!def) throw new Error(`rota desconhecida: ${id}`);
  return def;
}

export function isAvailable(id: RouteId): boolean {
  return routeDef(id).plannedPhase === null;
}
