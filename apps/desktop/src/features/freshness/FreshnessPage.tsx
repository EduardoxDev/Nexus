import { CheckCircle2, FolderGit2, History, RefreshCw, Undo2, X } from "lucide-react";
import { useState } from "react";

import { EmptyState, ErrorState } from "@/components/feedback";
import { ProjectPicker } from "@/components/ProjectPicker";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Segmented } from "@/components/ui/segmented";
import { Skeleton } from "@/components/ui/skeleton";
import { Tooltip } from "@/components/ui/tooltip";
import type { FreshnessIssue, IssueStatus } from "@/lib/bindings";
import { cn } from "@/lib/cn";
import { formatDateTime, formatDuration, formatRelative } from "@/lib/format";
import { t } from "@/lib/i18n/pt-BR";
import {
  useFreshnessIssues,
  useFreshnessMutations,
  useFreshnessSummary,
  useProjects,
} from "@/lib/queries";
import { useUi } from "@/stores/ui";

const s = t.freshnessPage;

export function FreshnessPage() {
  const projects = useProjects();
  const projectId = useUi((st) => st.selectedProjectId);
  const navigate = useUi((st) => st.navigate);
  const [status, setStatus] = useState<IssueStatus>("open");
  const summary = useFreshnessSummary(projectId);
  const issues = useFreshnessIssues(
    projectId === null ? null : { projectId, statuses: [status], limit: 500 },
  );
  const { scan } = useFreshnessMutations();

  if (projects.data?.length === 0) {
    return (
      <div className="flex h-full items-center justify-center">
        <EmptyState
          icon={FolderGit2}
          title={t.picker.none}
          description={t.picker.needProject}
          action={<Button onClick={() => navigate("projects")}>{t.picker.goProjects}</Button>}
        />
      </div>
    );
  }

  const sum = summary.data;
  const last = sum?.lastScan;
  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto flex w-full max-w-[1120px] flex-col gap-4 px-6 py-5">
        <header className="flex flex-wrap items-start justify-between gap-3">
          <div className="min-w-0">
            <h1 className="text-xl font-semibold tracking-tight">{s.title}</h1>
            <p className="mt-0.5 text-sm text-fg-muted">{t.nav.freshness.description}</p>
            <p className="mt-1 text-xs text-fg-muted">
              {last
                ? s.lastScan(
                    formatRelative(last.scannedAt),
                    last.docsChecked,
                    last.memoriesChecked,
                    formatDuration(last.durationMs),
                  )
                : s.neverScanned}
              {sum?.scanOutdated && last && <span className="text-warning"> · {s.outdated}</span>}
            </p>
          </div>
          <div className="flex items-center gap-2">
            <ProjectPicker />
            <Button
              variant="primary"
              onClick={() => projectId !== null && scan.mutate(projectId)}
              disabled={projectId === null || scan.isPending}
            >
              <RefreshCw aria-hidden="true" className={cn(scan.isPending && "animate-spin")} />
              {scan.isPending ? s.scanning : s.scan}
            </Button>
          </div>
        </header>
        {scan.isError && <ErrorState error={scan.error} />}

        {sum && (
          <dl className="grid grid-cols-5 gap-2" aria-label={s.title}>
            {(
              [
                [s.stats.open, sum.open],
                [s.stats.warnings, sum.warnings],
                [s.stats.contradictions, sum.contradictions],
                [s.stats.review, sum.needsReview],
                [s.stats.dismissed, sum.dismissed],
              ] as const
            ).map(([label, value]) => (
              <div key={label} className="rounded-lg border border-border bg-surface-1 px-3 py-2">
                <dt className="text-xs text-fg-muted">{label}</dt>
                <dd className="text-lg font-semibold tabular-nums">{value}</dd>
              </div>
            ))}
          </dl>
        )}

        <Segmented
          aria-label={s.title}
          value={status}
          onValueChange={setStatus}
          options={[
            { value: "open", label: s.filters.open },
            { value: "dismissed", label: s.filters.dismissed },
            { value: "resolved", label: s.filters.resolved },
          ]}
        />

        {issues.isError && <ErrorState error={issues.error} />}
        {issues.isPending && projectId !== null && <Skeleton className="h-32" />}
        {issues.data?.length === 0 && (
          <EmptyState
            icon={status === "open" ? CheckCircle2 : History}
            title={status === "open" ? s.emptyOpen : s.empty}
          />
        )}
        <div className="flex flex-col gap-3">
          {issues.data?.map((issue) => (
            <IssueCard key={issue.id} issue={issue} />
          ))}
        </div>
      </div>
    </div>
  );
}

/** Um alerta com motivo, fontes, recomendação e ações. Usado também na Memória e nos packs. */
export function IssueCard({
  issue,
  compact = false,
}: {
  issue: FreshnessIssue;
  compact?: boolean;
}) {
  const navigate = useUi((st) => st.navigate);
  const selectMemory = useUi((st) => st.selectMemory);
  const { dismiss } = useFreshnessMutations();
  const [noting, setNoting] = useState(false);
  const [note, setNote] = useState("");
  const openMemory = (id: string) => {
    selectMemory(id);
    navigate("memory");
  };
  const warning = issue.severity === "warning";

  return (
    <article
      aria-label={issue.title}
      className={cn(
        "rounded-lg border bg-surface-1 px-4 py-3",
        issue.contradiction ? "border-danger/50" : warning ? "border-warning/40" : "border-border",
      )}
    >
      <div className="flex flex-wrap items-center gap-1.5">
        {issue.contradiction && <Badge tone="danger">{s.contradiction}</Badge>}
        <Badge tone={warning ? "warning" : "info"}>{warning ? s.warning : s.info}</Badge>
        <Badge>{s.kinds[issue.kind] ?? issue.kind}</Badge>
        {issue.needsReview && (
          <Tooltip content={s.needsReviewHint}>
            <Badge tone="accent">{s.needsReview}</Badge>
          </Tooltip>
        )}
      </div>
      <h3 className="mt-1.5 text-sm font-semibold text-fg">{issue.title}</h3>
      <p className="mt-1 text-sm text-fg-muted">{issue.reason}</p>
      {issue.sources.length > 0 && (
        <ul className="mt-1.5 flex flex-wrap gap-x-3 gap-y-0.5" aria-label={s.sources}>
          {issue.sources.map((src) => (
            <li
              key={`${src.relPath}:${src.line ?? ""}:${src.cited ?? ""}`}
              className="font-mono text-xs"
            >
              {src.relPath}
              {src.line !== null && `:${src.line}`}
              {src.cited && issue.kind === "files_changed" && (
                <span className="text-fg-muted"> ({src.cited})</span>
              )}
            </li>
          ))}
        </ul>
      )}
      {issue.suggestion && (
        <p className="mt-1.5 text-sm">
          <span className="text-fg-muted">{s.suggestion}: </span>
          <span className="font-mono text-xs">{issue.suggestion}</span>
        </p>
      )}
      <p className="mt-1.5 text-sm">
        <span className="text-fg-muted">{s.recommendation}: </span>
        {issue.recommendation}
      </p>
      {!compact && (
        <p className="mt-1.5 text-xs text-fg-subtle">
          <span title={formatDateTime(issue.firstDetectedAt)}>
            {s.detected(
              formatRelative(issue.firstDetectedAt),
              formatRelative(issue.lastDetectedAt),
            )}
          </span>
          {issue.resolvedAt !== null && <> · {s.resolvedAt(formatRelative(issue.resolvedAt))}</>}
          {issue.dismissedAt !== null && (
            <>
              {" "}
              · {s.dismissedAt(formatRelative(issue.dismissedAt))}
              {issue.dismissNote && ` — ${issue.dismissNote}`}
            </>
          )}
        </p>
      )}
      <div className="mt-2 flex flex-wrap items-center gap-2">
        {!compact && issue.memoryId && (
          <Button size="sm" onClick={() => openMemory(issue.memoryId ?? "")}>
            {s.openMemory}
          </Button>
        )}
        {!compact && issue.relatedMemoryId && (
          <Button size="sm" onClick={() => openMemory(issue.relatedMemoryId ?? "")}>
            {s.openRelated}
          </Button>
        )}
        {issue.status === "open" && !noting && (
          <Button size="sm" variant="ghost" onClick={() => setNoting(true)}>
            <X aria-hidden="true" />
            {s.dismiss}
          </Button>
        )}
        {issue.status === "open" && noting && (
          <form
            className="flex items-center gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              dismiss.mutate({
                issueId: issue.id,
                dismissed: true,
                note: note.trim() ? note.trim() : null,
              });
            }}
          >
            <input
              value={note}
              onChange={(e) => setNote(e.target.value)}
              placeholder={s.dismissNote}
              aria-label={s.dismissNote}
              maxLength={1000}
              autoFocus
              className="h-7 w-64 rounded-md border border-border bg-surface-2 px-2 text-sm text-fg focus:border-accent focus:outline-none"
            />
            <Button size="sm" type="submit" disabled={dismiss.isPending}>
              {s.dismiss}
            </Button>
          </form>
        )}
        {issue.status === "dismissed" && (
          <Button
            size="sm"
            variant="ghost"
            onClick={() => dismiss.mutate({ issueId: issue.id, dismissed: false, note: null })}
            disabled={dismiss.isPending}
          >
            <Undo2 aria-hidden="true" />
            {s.reopen}
          </Button>
        )}
      </div>
      {dismiss.isError && <ErrorState className="mt-2" error={dismiss.error} />}
    </article>
  );
}
