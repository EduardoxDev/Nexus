import {
  FolderGit2,
  FolderOpen,
  Pause,
  Play,
  RefreshCw,
  RotateCcw,
  Square,
  Trash2,
  TriangleAlert,
} from "lucide-react";
import { useEffect, useState } from "react";

import { CopyButton, ErrorState } from "@/components/feedback";
import { KeyValueList, Panel } from "@/components/layout";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { Skeleton } from "@/components/ui/skeleton";
import { Tooltip } from "@/components/ui/tooltip";
import { jobFraction, jobProgressLabel } from "@/features/projects/activity";
import { AnalysisPanel } from "@/features/projects/AnalysisPanel";
import { JobsPanel } from "@/features/projects/JobsPanel";
import { LanguageBars } from "@/features/projects/LanguageBars";
import { RemoveProjectDialog } from "@/features/projects/RemoveProjectDialog";
import { SkippedPanel } from "@/features/projects/SkippedPanel";
import type { ProjectOverview } from "@/lib/bindings";
import {
  formatBytes,
  formatDateTime,
  formatInteger,
  formatRelative,
  shortHash,
} from "@/lib/format";
import { t } from "@/lib/i18n/pt-BR";
import { useIndexControl, useProjectOverview, useRevealProjectDir } from "@/lib/queries";
import { useProjectActivity } from "@/stores/indexing";
import { call } from "@/lib/ipc";

export function ProjectDetail({ projectId }: { projectId: number }) {
  const overview = useProjectOverview(projectId);

  // "Reabrir projetos recentes": registra o uso para ordenar a lista.
  useEffect(() => {
    void call("touch_project", { projectId }).catch(() => undefined);
  }, [projectId]);

  if (overview.isError) {
    return (
      <div className="p-6">
        <ErrorState error={overview.error} onRetry={() => void overview.refetch()} />
      </div>
    );
  }
  if (!overview.data) {
    return (
      <div className="flex flex-col gap-4 p-6" aria-busy="true" aria-label={t.common.loading}>
        <Skeleton className="h-8 w-1/3" />
        <Skeleton className="h-24" />
        <Skeleton className="h-48" />
      </div>
    );
  }
  return <ProjectDetailView overview={overview.data} />;
}

function ProjectDetailView({ overview }: { overview: ProjectOverview }) {
  const { summary } = overview;
  const { project, counts } = summary;
  const activity = useProjectActivity(project.id);
  const control = useIndexControl();
  const reveal = useRevealProjectDir();
  const [removing, setRemoving] = useState(false);
  const busy = activity.kind !== "idle";
  const controlError = control.start.error ?? control.pause.error ?? control.cancel.error;

  return (
    <div className="mx-auto flex w-full max-w-[1120px] flex-col gap-6 px-8 pt-7 pb-10">
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div className="flex min-w-0 items-center gap-4">
          <span
            className="flex size-12 shrink-0 items-center justify-center rounded-xl bg-accent-subtle text-accent-text ring-1 ring-accent/20"
            aria-hidden="true"
          >
            <FolderGit2 className="size-6" />
          </span>
          <div className="min-w-0">
            <h2 className="truncate font-display text-2xl font-semibold tracking-tight">
              {project.name}
            </h2>
            <div className="mt-0.5 flex items-center gap-1">
              <code className="selectable truncate font-mono text-xs text-fg-muted">
                {project.rootPath}
              </code>
              <CopyButton value={project.rootPath} />
            </div>
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button
            variant="primary"
            disabled={busy || !summary.rootExists}
            onClick={() => control.start.mutate({ projectId: project.id, mode: "incremental" })}
          >
            <RefreshCw aria-hidden="true" />
            {t.projects.actions.update}
          </Button>
          <Tooltip content={t.projects.actions.fullHint}>
            <Button
              disabled={busy || !summary.rootExists}
              onClick={() => control.start.mutate({ projectId: project.id, mode: "full" })}
            >
              <RotateCcw aria-hidden="true" />
              {t.projects.actions.full}
            </Button>
          </Tooltip>
          <Button
            variant="ghost"
            onClick={() => reveal.mutate(project.id)}
            disabled={!summary.rootExists}
          >
            <FolderOpen aria-hidden="true" />
            {t.projects.actions.reveal}
          </Button>
          <Button variant="ghost" onClick={() => setRemoving(true)}>
            <Trash2 aria-hidden="true" />
            {t.projects.actions.remove}
          </Button>
        </div>
      </header>

      {!summary.rootExists && (
        <div
          role="alert"
          className="flex gap-2 rounded-md border border-warning/40 bg-warning-subtle px-3 py-2 text-sm"
        >
          <TriangleAlert className="mt-0.5 size-4 shrink-0 text-warning" aria-hidden="true" />
          <span>
            {t.projects.rootMissing}: <code className="font-mono text-xs">{project.rootPath}</code>
          </span>
        </div>
      )}
      {controlError && <ErrorState error={controlError} />}
      {reveal.isError && <ErrorState error={reveal.error} />}

      {activity.kind === "running" && (
        <Panel>
          <div className="flex flex-wrap items-center gap-4">
            <div className="min-w-60 flex-1">
              <div className="mb-1.5 flex items-baseline justify-between gap-3 text-sm">
                <span className="font-medium">
                  {jobProgressLabel(activity.job, activity.paused)}
                </span>
                {activity.job && (
                  <span className="text-xs text-fg-muted">
                    {t.projects.liveCounts(
                      formatInteger(activity.job.unchanged),
                      formatInteger(activity.job.skipped),
                      formatBytes(activity.job.bytesRead),
                    )}
                  </span>
                )}
              </div>
              <Progress
                value={jobFraction(activity.job)}
                label={jobProgressLabel(activity.job, activity.paused)}
              />
            </div>
            <div className="flex gap-2">
              {activity.paused ? (
                <Button size="sm" onClick={() => control.resume.mutate(project.id)}>
                  <Play aria-hidden="true" />
                  {t.projects.actions.resume}
                </Button>
              ) : (
                <Button size="sm" onClick={() => control.pause.mutate(project.id)}>
                  <Pause aria-hidden="true" />
                  {t.projects.actions.pause}
                </Button>
              )}
              <Button size="sm" variant="ghost" onClick={() => control.cancel.mutate(project.id)}>
                <Square aria-hidden="true" />
                {t.projects.actions.cancel}
              </Button>
            </div>
          </div>
        </Panel>
      )}
      {activity.kind === "queued" && (
        <p className="text-sm text-fg-muted" aria-live="polite">
          {t.projects.queued(activity.position)}
        </p>
      )}

      <dl className="grid grid-cols-2 gap-px overflow-hidden rounded-xl border border-border bg-border [box-shadow:var(--shadow-card)] lg:grid-cols-4">
        <Stat label={t.projects.stats.indexed} value={formatInteger(counts.indexed)} />
        <Stat label={t.projects.stats.skipped} value={formatInteger(counts.skipped)} />
        <Stat
          label={t.projects.stats.failed}
          value={formatInteger(counts.failed)}
          tone={counts.failed > 0 ? "danger" : undefined}
        />
        <Stat label={t.projects.stats.size} value={formatBytes(counts.indexedBytes)} />
        <Stat label={t.projects.stats.lines} value={formatInteger(counts.indexedLines)} />
        <Stat label={t.projects.stats.generation} value={String(project.indexGeneration)} />
        <Stat
          label={t.projects.stats.commit}
          value={
            project.headCommit
              ? `${project.headRef?.replace("refs/heads/", "") ?? t.projects.stats.detached} · ${shortHash(project.headCommit, 8)}`
              : t.projects.stats.noGit
          }
          mono={project.headCommit !== null}
        />
        <Stat
          label={t.projects.stats.lastIndexed}
          value={
            project.lastIndexedAt === null ? t.common.never : formatRelative(project.lastIndexedAt)
          }
          title={project.lastIndexedAt === null ? undefined : formatDateTime(project.lastIndexedAt)}
        />
      </dl>

      <div className="grid gap-5 xl:grid-cols-2">
        <Panel title={t.projects.languages} description={t.projects.languagesDescription}>
          <LanguageBars languages={overview.languages} />
        </Panel>
        <Panel title={t.projects.kinds}>
          {overview.kinds.length === 0 ? (
            <p className="text-sm text-fg-muted">{t.projects.none}</p>
          ) : (
            <KeyValueList
              items={overview.kinds.map((k) => ({
                label: t.fileKinds[k.kind],
                value: formatInteger(k.files),
              }))}
            />
          )}
        </Panel>
        <Panel title={t.projects.structure} description={t.projects.structureDescription}>
          {overview.topDirectories.length === 0 ? (
            <p className="text-sm text-fg-muted">{t.projects.none}</p>
          ) : (
            <KeyValueList
              items={overview.topDirectories.map((d) => ({
                label: d.name === "" ? t.projects.rootFiles : `${d.name}/`,
                value: formatInteger(d.files),
              }))}
            />
          )}
        </Panel>
        <Panel title={t.projects.manifests}>
          <PathList paths={overview.manifests} />
          <h3 className="mt-4 mb-1.5 text-sm font-semibold">{t.projects.ci}</h3>
          <PathList paths={overview.ciFiles} />
        </Panel>
      </div>

      <Panel title={t.projects.mostChanged} description={t.projects.mostChangedDescription}>
        {overview.mostChanged.length === 0 ? (
          <p className="text-sm text-fg-muted">{t.projects.mostChangedEmpty}</p>
        ) : (
          <KeyValueList
            items={overview.mostChanged.map((f) => ({
              label: t.projects.changes(f.changeCount),
              value: f.relPath,
              mono: true,
            }))}
          />
        )}
      </Panel>

      <AnalysisPanel projectId={project.id} />
      <SkippedPanel projectId={project.id} />
      <JobsPanel projectId={project.id} />

      <RemoveProjectDialog
        open={removing}
        onOpenChange={setRemoving}
        projectId={project.id}
        name={project.name}
      />
    </div>
  );
}

function Stat({
  label,
  value,
  tone,
  mono,
  title,
}: {
  label: string;
  value: string;
  tone?: "danger" | undefined;
  mono?: boolean;
  title?: string | undefined;
}) {
  return (
    // Valor inteiro também no tooltip (hash e textos podem não caber na coluna).
    <div className="min-w-0 bg-surface-1 px-5 py-3.5" title={title ?? value}>
      <dt className="text-[11px] font-semibold tracking-[0.06em] text-fg-subtle uppercase">
        {label}
      </dt>
      <dd
        className={
          (tone === "danger" ? "text-danger " : "text-fg ") +
          (mono
            ? "mt-1 truncate font-mono text-sm"
            : /^[\d.,\s]+[A-Za-z]*$/.test(value)
              ? "mt-1 truncate font-display text-xl font-semibold tabular-nums"
              : // Texto quebra linha em vez de ser cortado ("Não é um repositório git").
                "mt-1 font-display text-base leading-snug font-semibold text-balance break-words")
        }
      >
        {value}
      </dd>
    </div>
  );
}

function PathList({ paths }: { paths: string[] }) {
  if (paths.length === 0) return <p className="text-sm text-fg-muted">{t.projects.none}</p>;
  return (
    <ul className="flex flex-wrap gap-1.5">
      {paths.map((p) => (
        <li key={p}>
          <Badge className="font-mono">{p}</Badge>
        </li>
      ))}
    </ul>
  );
}
