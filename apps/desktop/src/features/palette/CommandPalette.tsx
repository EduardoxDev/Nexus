import { Command } from "cmdk";
import {
  FileUp,
  FolderGit2,
  FolderOpen,
  FolderPlus,
  Keyboard,
  Monitor,
  Moon,
  RefreshCw,
  ScanSearch,
  ShieldCheck,
  Sun,
} from "lucide-react";
import type { ReactNode } from "react";

import { ROUTES } from "@/app/routes";
import { Badge } from "@/components/ui/badge";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import type { ThemePreference } from "@/lib/bindings";
import { t } from "@/lib/i18n/pt-BR";
import {
  useAddProject,
  useIndexControl,
  useFreshnessMutations,
  useIntegrityCheck,
  useMemoryMutations,
  useProjects,
  useRevealDataDir,
  useSetSetting,
} from "@/lib/queries";
import { useUi } from "@/stores/ui";

export function CommandPalette() {
  const open = useUi((s) => s.paletteOpen);
  const { setPaletteOpen, navigate, setShortcutsOpen, selectProject } = useUi.getState();
  const projects = useProjects();
  const addProject = useAddProject();
  const control = useIndexControl();
  const setSetting = useSetSetting();
  const integrity = useIntegrityCheck();
  const reveal = useRevealDataDir();
  const { scan } = useFreshnessMutations();
  const { importAdrs } = useMemoryMutations();

  const run = (action: () => void) => {
    setPaletteOpen(false);
    action();
  };
  const setTheme = (theme: ThemePreference) =>
    run(() => setSetting.mutate({ key: "ui.theme", value: theme }));

  return (
    <Dialog open={open} onOpenChange={setPaletteOpen}>
      <DialogContent showClose={false} aria-describedby={undefined} className="overflow-hidden p-0">
        <DialogTitle className="sr-only">{t.palette.title}</DialogTitle>
        <Command label={t.palette.title} loop className="flex flex-col">
          <Command.Input
            autoFocus
            placeholder={t.palette.placeholder}
            className="h-11 border-b border-border bg-transparent px-4 text-base text-fg outline-none placeholder:text-fg-subtle"
          />
          <Command.List className="max-h-[360px] overflow-y-auto p-1.5">
            <Command.Empty className="px-3 py-6 text-center text-sm text-fg-muted">
              {t.palette.empty}
            </Command.Empty>

            <Group heading={t.palette.groupNavigation}>
              {ROUTES.map((route) => {
                const Icon = route.icon;
                const disabled = route.plannedPhase !== null;
                return (
                  <Item
                    key={route.id}
                    value={`nav ${route.label}`}
                    disabled={disabled}
                    onSelect={() => run(() => navigate(route.id))}
                  >
                    <Icon aria-hidden="true" />
                    <span className="flex-1">{route.label}</span>
                    {route.plannedPhase !== null && (
                      <Badge>{t.nav.phaseBadge(route.plannedPhase)}</Badge>
                    )}
                  </Item>
                );
              })}
            </Group>

            {projects.data && projects.data.length > 0 && (
              <Group heading={t.palette.groupProjects}>
                {projects.data.map((p) => (
                  <Item
                    key={`open-${p.project.id}`}
                    value={t.projects.paletteOpen(p.project.name)}
                    onSelect={() =>
                      run(() => {
                        selectProject(p.project.id);
                        navigate("projects");
                      })
                    }
                  >
                    <FolderGit2 aria-hidden="true" />
                    {t.projects.paletteOpen(p.project.name)}
                  </Item>
                ))}
                {projects.data.map((p) => (
                  <Item
                    key={`update-${p.project.id}`}
                    value={t.projects.paletteUpdate(p.project.name)}
                    onSelect={() =>
                      run(() =>
                        control.start.mutate({ projectId: p.project.id, mode: "incremental" }),
                      )
                    }
                  >
                    <RefreshCw aria-hidden="true" />
                    {t.projects.paletteUpdate(p.project.name)}
                  </Item>
                ))}
                {projects.data.map((p) => (
                  <Item
                    key={`fresh-${p.project.id}`}
                    value={t.projects.paletteFreshness(p.project.name)}
                    onSelect={() =>
                      run(() => {
                        selectProject(p.project.id);
                        navigate("freshness");
                        scan.mutate(p.project.id);
                      })
                    }
                  >
                    <ScanSearch aria-hidden="true" />
                    {t.projects.paletteFreshness(p.project.name)}
                  </Item>
                ))}
                {projects.data.map((p) => (
                  <Item
                    key={`adrs-${p.project.id}`}
                    value={t.projects.paletteImportAdrs(p.project.name)}
                    onSelect={() =>
                      run(() => {
                        selectProject(p.project.id);
                        navigate("memory");
                        importAdrs.mutate(p.project.id);
                      })
                    }
                  >
                    <FileUp aria-hidden="true" />
                    {t.projects.paletteImportAdrs(p.project.name)}
                  </Item>
                ))}
              </Group>
            )}

            <Group heading={t.palette.groupActions}>
              <Item
                value={t.projects.add}
                onSelect={() =>
                  run(() => {
                    navigate("projects");
                    addProject.mutate(undefined, {
                      onSuccess: (summary) => {
                        if (summary) selectProject(summary.project.id);
                      },
                    });
                  })
                }
              >
                <FolderPlus aria-hidden="true" />
                {t.projects.add}
              </Item>
              <Item
                value={t.palette.runIntegrity}
                onSelect={() =>
                  run(() => {
                    navigate("diagnostics");
                    integrity.mutate();
                  })
                }
              >
                <ShieldCheck aria-hidden="true" />
                {t.palette.runIntegrity}
              </Item>
              <Item value={t.palette.revealData} onSelect={() => run(() => reveal.mutate())}>
                <FolderOpen aria-hidden="true" />
                {t.palette.revealData}
              </Item>
              <Item
                value={t.palette.showShortcuts}
                onSelect={() => run(() => setShortcutsOpen(true))}
              >
                <Keyboard aria-hidden="true" />
                {t.palette.showShortcuts}
              </Item>
            </Group>

            <Group heading={t.palette.groupAppearance}>
              <Item value={t.palette.themeSystem} onSelect={() => setTheme("system")}>
                <Monitor aria-hidden="true" />
                {t.palette.themeSystem}
              </Item>
              <Item value={t.palette.themeDark} onSelect={() => setTheme("dark")}>
                <Moon aria-hidden="true" />
                {t.palette.themeDark}
              </Item>
              <Item value={t.palette.themeLight} onSelect={() => setTheme("light")}>
                <Sun aria-hidden="true" />
                {t.palette.themeLight}
              </Item>
            </Group>
          </Command.List>
        </Command>
      </DialogContent>
    </Dialog>
  );
}

function Group({ heading, children }: { heading: string; children: ReactNode }) {
  return (
    <Command.Group
      heading={heading}
      className="[&_[cmdk-group-heading]]:px-2.5 [&_[cmdk-group-heading]]:pt-2 [&_[cmdk-group-heading]]:pb-1 [&_[cmdk-group-heading]]:text-xs [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:text-fg-subtle"
    >
      {children}
    </Command.Group>
  );
}

function Item({
  children,
  value,
  disabled,
  onSelect,
}: {
  children: ReactNode;
  value: string;
  disabled?: boolean;
  onSelect: () => void;
}) {
  return (
    <Command.Item
      value={value}
      onSelect={onSelect}
      {...(disabled === undefined ? {} : { disabled })}
      className="flex h-9 cursor-default items-center gap-2.5 rounded-md px-2.5 text-sm text-fg data-[disabled=true]:text-fg-subtle data-[selected=true]:bg-surface-3 [&_svg]:size-4 [&_svg]:text-fg-muted"
    >
      {children}
    </Command.Item>
  );
}
