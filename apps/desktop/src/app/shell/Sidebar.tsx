import { PanelLeftClose, PanelLeftOpen, ShieldCheck } from "lucide-react";
import { motion } from "motion/react";

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
        "flex h-full shrink-0 flex-col border-r border-border bg-surface-sidebar transition-[width] duration-[var(--duration-base)]",
        collapsed ? "w-[var(--sidebar-width-collapsed)]" : "w-[var(--sidebar-width)]",
      )}
    >
      <div
        className={cn("flex h-14 items-center gap-2.5 px-4", collapsed && "justify-center px-0")}
      >
        <Logo />
        {!collapsed && (
          <div className="min-w-0 leading-tight">
            <p className="font-display text-sm font-bold tracking-[0.14em] text-fg">{t.app.name}</p>
            <p className="truncate text-[11px] text-fg-subtle">{t.app.edition}</p>
          </div>
        )}
      </div>

      <div className="flex flex-1 flex-col gap-5 overflow-y-auto px-2.5 py-3">
        <NavGroup label={t.nav.sectionWorkspace} routes={workspace} collapsed={collapsed} />
        <NavGroup label={t.nav.sectionSystem} routes={system} collapsed={collapsed} />
      </div>

      {!collapsed && (
        <div className="mx-2.5 mb-2 flex items-start gap-2.5 rounded-lg border border-border bg-surface-1 px-3 py-2.5 [box-shadow:var(--shadow-card)]">
          <ShieldCheck className="mt-px size-4 shrink-0 text-success" aria-hidden="true" />
          <div className="min-w-0">
            <p className="text-xs font-semibold text-fg">{t.nav.privacyTitle}</p>
            <p className="text-[11px] leading-4 text-fg-subtle">{t.nav.privacyBody}</p>
          </div>
        </div>
      )}

      <div className={cn("border-t border-border p-2.5", collapsed && "flex justify-center")}>
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
      {!collapsed && (
        <p className="px-2.5 pb-1.5 text-[11px] font-semibold tracking-[0.08em] text-fg-subtle uppercase">
          {label}
        </p>
      )}
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
        "group relative flex h-9 w-full items-center gap-2.5 rounded-lg px-2.5 text-left text-sm transition-colors",
        collapsed && "justify-center px-0",
        active ? "font-medium text-fg" : "text-fg-muted hover:bg-surface-2 hover:text-fg",
        disabled && "cursor-default text-fg-subtle hover:bg-transparent hover:text-fg-subtle",
      )}
    >
      {active && (
        <motion.span
          layoutId="nav-active"
          aria-hidden="true"
          transition={{ type: "spring", stiffness: 520, damping: 40 }}
          className="absolute inset-0 rounded-lg bg-surface-1 ring-1 ring-border [box-shadow:var(--shadow-card)]"
        >
          <span className="absolute top-2 bottom-2 -left-2.5 w-[3px] rounded-r-full bg-accent" />
        </motion.span>
      )}
      <Icon
        className={cn(
          "relative size-4 shrink-0 transition-transform duration-[var(--duration-fast)] group-hover:scale-110",
          active ? "text-accent-text" : "",
        )}
        aria-hidden="true"
      />
      {!collapsed && <span className="relative flex-1 truncate">{route.label}</span>}
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
    <img
      src={logoUrl}
      alt=""
      aria-hidden="true"
      className="size-8 shrink-0 rounded-[9px] shadow-card"
      draggable={false}
    />
  );
}
