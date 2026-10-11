import { FolderOpen, Monitor, Moon, ShieldCheck, Sun } from "lucide-react";
import type { ReactNode } from "react";

import { ErrorState } from "@/components/feedback";
import { Page, Panel } from "@/components/layout";
import { Button } from "@/components/ui/button";
import { Segmented, type SegmentedOption } from "@/components/ui/segmented";
import { Skeleton } from "@/components/ui/skeleton";
import type { MotionPreference, ThemePreference } from "@/lib/bindings";
import { t } from "@/lib/i18n/pt-BR";
import { useRevealDataDir, useSetSetting, useSettings, useStartupStatus } from "@/lib/queries";

const THEME_OPTIONS: readonly SegmentedOption<ThemePreference>[] = [
  { value: "system", label: t.settings.themeOptions.system, icon: Monitor },
  { value: "dark", label: t.settings.themeOptions.dark, icon: Moon },
  { value: "light", label: t.settings.themeOptions.light, icon: Sun },
];

const MOTION_OPTIONS: readonly SegmentedOption<MotionPreference>[] = [
  { value: "system", label: t.settings.motionOptions.system },
  { value: "on", label: t.settings.motionOptions.on },
  { value: "off", label: t.settings.motionOptions.off },
];

export function SettingsPage() {
  const settings = useSettings();
  const setSetting = useSetSetting();
  const startup = useStartupStatus();
  const reveal = useRevealDataDir();

  return (
    <Page title={t.nav.settings.label} description={t.nav.settings.description}>
      <Panel title={t.settings.appearance}>
        {settings.isError ? (
          <ErrorState error={settings.error} onRetry={() => void settings.refetch()} />
        ) : !settings.data ? (
          <div className="flex flex-col gap-3" aria-busy="true">
            <Skeleton className="h-8 w-80" />
            <Skeleton className="h-8 w-80" />
          </div>
        ) : (
          <div className="flex flex-col divide-y divide-border">
            <SettingRow label={t.settings.theme} description={t.settings.themeDescription}>
              <Segmented
                aria-label={t.settings.theme}
                value={settings.data.theme}
                options={THEME_OPTIONS}
                disabled={setSetting.isPending}
                onValueChange={(value) => setSetting.mutate({ key: "ui.theme", value })}
              />
            </SettingRow>
            <SettingRow label={t.settings.motion} description={t.settings.motionDescription}>
              <Segmented
                aria-label={t.settings.motion}
                value={settings.data.reducedMotion}
                options={MOTION_OPTIONS}
                disabled={setSetting.isPending}
                onValueChange={(value) => setSetting.mutate({ key: "ui.reduced_motion", value })}
              />
            </SettingRow>
          </div>
        )}
        {setSetting.isError && <ErrorState className="mt-3" error={setSetting.error} />}
      </Panel>

      <Panel title={t.settings.data} description={t.settings.dataDescription}>
        <div className="flex items-center gap-3">
          <code className="min-w-0 flex-1 truncate rounded-md border border-border bg-surface-2 px-2.5 py-1.5 font-mono text-xs text-fg">
            {startup.data?.dataDir ?? t.common.unknown}
          </code>
          <Button onClick={() => reveal.mutate()} disabled={!startup.data?.dataDir}>
            <FolderOpen aria-hidden="true" />
            {t.settings.reveal}
          </Button>
        </div>
        {reveal.isError && <ErrorState className="mt-3" error={reveal.error} />}
      </Panel>

      <Panel title={t.settings.privacy}>
        <ul className="flex flex-col gap-2">
          {t.settings.privacyItems.map((item) => (
            <li key={item} className="flex gap-2.5 text-sm text-fg">
              <ShieldCheck className="mt-0.5 size-4 shrink-0 text-success" aria-hidden="true" />
              {item}
            </li>
          ))}
        </ul>
      </Panel>
    </Page>
  );
}

function SettingRow({
  label,
  description,
  children,
}: {
  label: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-x-6 gap-y-2 py-3 first:pt-0 last:pb-0">
      <div className="min-w-0 max-w-md">
        <p className="text-sm font-medium text-fg">{label}</p>
        <p className="text-xs text-fg-muted">{description}</p>
      </div>
      {children}
    </div>
  );
}
