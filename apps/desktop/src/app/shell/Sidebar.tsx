import { PanelLeftClose, PanelLeftOpen } from "lucide-react";

import { ROUTES, type RouteDef } from "@/app/routes";
import logoUrl from "@/assets/nexus-icon-128.png";
import { Badge } from "@/components/ui/badge";
import { Tooltip } from "@/components/ui/tooltip";
import { cn } from "@/lib/cn";
import { t } from "@/lib/i18n/pt-BR";
import { useUi } from "@/stores/ui";

interface SidebarProps {
  collapsed: boolean;
  onToggleCollapsed: () => void;
}

export function Sidebar({ collapsed, onToggleCollapsed }: SidebarProps) {
  const workspace = ROUTES.filter((r) => r.group === "workspace");
  const system = ROUTES.filter((r) => r.group === "system");
  return (
    <nav
      aria-label="Navegação principal"
      className={cn(
        "flex h-full shrink-0 flex-col border-r border-border bg-surface-1 transition-[width] duration-[var(--duration-base)]",
        collapsed ? "w-[var(--sidebar-width-collapsed)]" : "w-[var(--sidebar-width)]",
      )}
    >
      <div
        className={cn("flex h-12 items-center gap-2.5 px-4", collapsed && "justify-center px-0")}
      >
        <Logo />
        {!collapsed && (
          <span className="text-sm font-semibold tracking-[0.12em] text-fg">{t.app.name}</span>
        )}
      </div>

      <div className="flex flex-1 flex-col gap-4 overflow-y-auto px-2 py-2">
        <NavGroup label={t.nav.sectionWorkspace} routes={workspace} collapsed={collapsed} />
        <NavGroup label={t.nav.sectionSystem} routes={system} collapsed={collapsed} />
      </div>

      <div className={cn("border-t border-border p-2", collapsed && "flex justify-center")}>
        <Tooltip content={collapsed ? t.nav.expand : t.nav.collapse} side="right">
          <button
            type="button"
            onClick={onToggleCollapsed}
            aria-label={collapsed ? t.nav.expand : t.nav.collapse}
            className="flex h-8 w-full items-center gap-2 rounded-md px-2 text-sm text-fg-subtle hover:bg-surface-2 hover:text-fg"
          >
            {collapsed ? (
              <PanelLeftOpen className="size-4" aria-hidden="true" />
            ) : (
              <PanelLeftClose className="size-4" aria-hidden="true" />
            )}
            {!collapsed && <span>{t.nav.collapse}</span>}
          </button>
        </Tooltip>
      </div>
    </nav>
  );
}

function NavGroup({
  label,
  routes,
  collapsed,
}: {
  label: string;
  routes: RouteDef[];
  collapsed: boolean;
}) {
  return (
    <div>
      {!collapsed && <p className="px-2 pb-1 text-xs font-medium text-fg-subtle">{label}</p>}
      <ul className="flex flex-col gap-0.5">
        {routes.map((route) => (
          <li key={route.id}>
            <NavItem route={route} collapsed={collapsed} />
          </li>
        ))}
      </ul>
    </div>
  );
}

function NavItem({ route, collapsed }: { route: RouteDef; collapsed: boolean }) {
  const current = useUi((s) => s.route);
  const navigate = useUi((s) => s.navigate);
  const Icon = route.icon;
  const disabled = route.plannedPhase !== null;
  const active = current === route.id;

  const item = (
    <button
      type="button"
      onClick={() => navigate(route.id)}
      aria-current={active ? "page" : undefined}
      aria-disabled={disabled || undefined}
      aria-label={collapsed ? route.label : undefined}
      className={cn(
        "group relative flex h-8 w-full items-center gap-2.5 rounded-md px-2 text-left text-sm transition-colors",
        collapsed && "justify-center px-0",
        active
          ? "bg-accent-subtle font-medium text-fg"
          : "text-fg-muted hover:bg-surface-2 hover:text-fg",
        disabled && "cursor-default text-fg-subtle hover:bg-transparent hover:text-fg-subtle",
      )}
    >
      {active && (
        <span
          aria-hidden="true"
          className="absolute top-1.5 bottom-1.5 left-0 w-0.5 rounded-full bg-accent"
        />
      )}
      <Icon
        className={cn("size-4 shrink-0", active ? "text-accent-text" : "")}
        aria-hidden="true"
      />
      {!collapsed && <span className="flex-1 truncate">{route.label}</span>}
      {!collapsed && disabled && route.plannedPhase !== null && (
        <Badge className="text-[11px]">{t.nav.phaseBadge(route.plannedPhase)}</Badge>
      )}
    </button>
  );

  if (disabled && route.plannedPhase !== null) {
    return (
      <Tooltip
        side="right"
        content={
          <span>
            <strong className="font-medium">{route.label}</strong> —{" "}
            {t.nav.phaseTooltip(route.plannedPhase)}
          </span>
        }
      >
        {item}
      </Tooltip>
    );
  }
  if (collapsed) {
    return (
      <Tooltip side="right" content={route.label}>
        {item}
      </Tooltip>
    );
  }
  return item;
}

function Logo() {
  return (
    <img src={logoUrl} alt="" aria-hidden="true" className="size-6 shrink-0" draggable={false} />
  );
}
