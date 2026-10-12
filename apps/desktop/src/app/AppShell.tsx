import { AnimatePresence, MotionConfig, motion } from "motion/react";
import { useCallback, type ReactNode } from "react";

import type { RouteId } from "@/app/routes";
import { Header } from "@/app/shell/Header";
import { Sidebar } from "@/app/shell/Sidebar";
import { StatusBar } from "@/app/shell/StatusBar";
import { useGlobalShortcuts } from "@/app/useGlobalShortcuts";
import { DiagnosticsPage } from "@/features/diagnostics/DiagnosticsPage";
import { OverviewPage } from "@/features/overview/OverviewPage";
import { FreshnessPage } from "@/features/freshness/FreshnessPage";
import { MemoryPage } from "@/features/memory/MemoryPage";
import { ContextPacksPage } from "@/features/packs/ContextPacksPage";
import { ProjectsPage } from "@/features/projects/ProjectsPage";
import { SearchPage } from "@/features/search/SearchPage";
import { CommandPalette } from "@/features/palette/CommandPalette";
import { ShortcutsDialog } from "@/features/palette/ShortcutsDialog";
import { SettingsPage } from "@/features/settings/SettingsPage";
import type { Settings } from "@/lib/bindings";
import { useIndexingEvents } from "@/lib/events";
import { useSetSetting } from "@/lib/queries";
import { useApplyAppearance } from "@/lib/theme";
import { useUi } from "@/stores/ui";

/** Páginas disponíveis. Seções futuras não têm página (a navegação as recusa). */
const PAGES: Partial<Record<RouteId, () => ReactNode>> = {
  overview: () => <OverviewPage />,
  projects: () => <ProjectsPage />,
  search: () => <SearchPage />,
  contextPacks: () => <ContextPacksPage />,
  memory: () => <MemoryPage />,
  freshness: () => <FreshnessPage />,
  settings: () => <SettingsPage />,
  diagnostics: () => <DiagnosticsPage />,
};

/** Páginas que gerenciam a própria rolagem (layout em colunas). */
const FULL_HEIGHT = new Set<RouteId>([
  "diagnostics",
  "projects",
  "search",
  "contextPacks",
  "memory",
  "freshness",
]);

export function AppShell({ settings }: { settings: Settings }) {
  const route = useUi((s) => s.route);
  const setSetting = useSetSetting();
  const appearance = useApplyAppearance(settings.theme, settings.reducedMotion);
  useIndexingEvents();

  const toggleSidebar = useCallback(() => {
    setSetting.mutate({ key: "ui.sidebar_collapsed", value: !settings.sidebarCollapsed });
  }, [setSetting, settings.sidebarCollapsed]);
  useGlobalShortcuts({ onToggleSidebar: toggleSidebar });

  const page = PAGES[route]?.() ?? null;

  return (
    <MotionConfig reducedMotion={appearance.reducedMotion ? "always" : "never"}>
      <div className="flex h-full flex-col">
        <div className="flex min-h-0 flex-1">
          <Sidebar collapsed={settings.sidebarCollapsed} onToggleCollapsed={toggleSidebar} />
          <div className="flex min-w-0 flex-1 flex-col">
            <Header
              resolvedTheme={appearance.theme}
              onToggleTheme={() =>
                setSetting.mutate({
                  key: "ui.theme",
                  value: appearance.theme === "dark" ? "light" : "dark",
                })
              }
            />
            <main id="main" className="relative min-h-0 flex-1 overflow-hidden">
              <AnimatePresence mode="wait" initial={false}>
                <motion.div
                  key={route}
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  transition={{ duration: 0.12, ease: "easeOut" }}
                  className={FULL_HEIGHT.has(route) ? "h-full" : "h-full overflow-y-auto"}
                >
                  {page}
                </motion.div>
              </AnimatePresence>
            </main>
          </div>
        </div>
        <StatusBar />
      </div>
      <CommandPalette />
      <ShortcutsDialog />
    </MotionConfig>
  );
}
