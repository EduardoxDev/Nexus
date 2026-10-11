import { FolderOpen, Monitor, Moon, ShieldCheck, Sun } from "lucide-react";
import type { ReactNode } from "react";

import { ErrorState } from "@/components/feedback";
import { Page, Panel } from "@/components/layout";
import { Button } from "@/components/ui/button";
import { Segmented, type SegmentedOption } from "@/components/ui/segmented";
import { Skeleton } from "@/components/ui/skeleton";
import type { MotionPreference, ThemePreference } from "@/lib/bindings";
import { formatBytes } from "@/lib/format";
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

const THREAD_OPTIONS = [
  { value: "0", label: t.indexingSettings.threadsAuto },
  { value: "1", label: "1" },
  { value: "2", label: "2" },
  { value: "4", label: "4" },
  { value: "8", label: "8" },
  { value: "16", label: "16" },
];

const SIZE_OPTIONS = [256, 512, 1024, 2048, 5120, 10240].map((kb) => ({
  value: String(kb),
  label: formatBytes(kb * 1024),
}));

const WATCH_OPTIONS: readonly SegmentedOption<"on" | "off">[] = [
  { value: "on", label: t.indexingSettings.watchOn },
  { value: "off", label: t.indexingSettings.watchOff },
];

function NativeSelect({
  label,
  value,
  options,
  disabled,
  onChange,
}: {
  label: string;
  value: string;
  options: { value: string; label: string }[];
  disabled?: boolean;
  onChange: (value: string) => void;
}) {
  // Valores fora da lista (ex.: definidos pela CLI) continuam visíveis.
  const all = options.some((o) => o.value === value)
    ? options
    : [...options, { value, label: value }];
  return (
    <select
      aria-label={label}
      value={value}
      disabled={disabled}
      onChange={(e) => onChange(e.target.value)}
      className="h-8 rounded-md border border-border bg-surface-2 px-2 text-sm text-fg hover:border-border-strong disabled:opacity-50"
    >
      {all.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}

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

      {settings.data && (
        <Panel title={t.indexingSettings.title}>
          <div className="flex flex-col divide-y divide-border">
            <SettingRow
              label={t.indexingSettings.threads}
              description={t.indexingSettings.threadsDescription}
            >
              <NativeSelect
                label={t.indexingSettings.threads}
                value={String(settings.data.indexingThreads)}
                options={THREAD_OPTIONS}
                disabled={setSetting.isPending}
                onChange={(v) => setSetting.mutate({ key: "indexing.threads", value: Number(v) })}
              />
            </SettingRow>
            <SettingRow
              label={t.indexingSettings.maxSize}
              description={t.indexingSettings.maxSizeDescription}
            >
              <NativeSelect
                label={t.indexingSettings.maxSize}
                value={String(settings.data.maxFileSizeKb)}
                options={SIZE_OPTIONS}
                disabled={setSetting.isPending}
                onChange={(v) =>
                  setSetting.mutate({ key: "indexing.max_file_size_kb", value: Number(v) })
                }
              />
            </SettingRow>
            <SettingRow
              label={t.indexingSettings.watch}
              description={t.indexingSettings.watchDescription}
            >
              <Segmented
                aria-label={t.indexingSettings.watch}
                value={settings.data.watchProjects ? "on" : "off"}
                options={WATCH_OPTIONS}
                disabled={setSetting.isPending}
                onValueChange={(v) =>
                  setSetting.mutate({ key: "indexing.watch", value: v === "on" })
                }
              />
            </SettingRow>
          </div>
        </Panel>
      )}

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
