import {
  Archive,
  ArchiveRestore,
  Brain,
  CheckCheck,
  FileUp,
  FolderUp,
  FolderGit2,
  Pencil,
  Plus,
  RefreshCcw,
  TriangleAlert,
} from "lucide-react";
import { useState } from "react";

import { EmptyState, ErrorState } from "@/components/feedback";
import { Panel } from "@/components/layout";
import { ProjectPicker } from "@/components/ProjectPicker";
import { Badge, type BadgeProps } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Tooltip } from "@/components/ui/tooltip";
import { IssueCard } from "@/features/freshness/FreshnessPage";
import { MEMORY_KINDS, MemoryEditor } from "@/features/memory/MemoryEditor";
import { MemoryExportDialog } from "@/features/memory/MemoryExportDialog";
import type { MemoryDetail, MemoryKind, MemoryStatus, RefState } from "@/lib/bindings";
import { cn } from "@/lib/cn";
import { formatDateTime, formatRelative } from "@/lib/format";
import { t } from "@/lib/i18n/pt-BR";
import {
  useFreshnessIssues,
  useMemories,
  useMemory,
  useMemoryCounts,
  useMemoryMutations,
  useProjects,
} from "@/lib/queries";
import { useUi } from "@/stores/ui";

const TABS: MemoryStatus[] = ["proposed", "confirmed", "archived", "superseded"];

export const statusTone: Record<MemoryStatus, BadgeProps["tone"]> = {
  proposed: "warning",
  confirmed: "success",
  archived: "neutral",
  superseded: "neutral",
};

const refTone: Record<RefState, BadgeProps["tone"]> = {
  unchanged: "success",
  changed: "warning",
  missing: "danger",
  moved: "warning",
  not_indexed: "warning",
};

export function MemoryPage() {
  const projects = useProjects();
  const projectId = useUi((s) => s.selectedProjectId);
  const navigate = useUi((s) => s.navigate);
  const selectedId = useUi((s) => s.selectedMemoryId);
  const selectMemory = useUi((s) => s.selectMemory);
  const [tab, setTab] = useState<MemoryStatus>("proposed");
  const [kind, setKind] = useState<MemoryKind | "all">("all");
  const [text, setText] = useState("");
  const [creating, setCreating] = useState(false);
  const [exporting, setExporting] = useState(false);
  const counts = useMemoryCounts(projectId);
  const list = useMemories(
    projectId === null
      ? null
      : {
          projectId,
          statuses: [tab],
          ...(kind === "all" ? {} : { kinds: [kind] }),
          ...(text.trim() ? { text: text.trim() } : {}),
          limit: 200,
        },
  );
  const { importAdrs } = useMemoryMutations();

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

  const report = importAdrs.data;
  return (
    <div className="flex h-full min-h-0">
      <aside className="flex w-80 shrink-0 flex-col border-r border-border bg-surface-sidebar">
        <div className="flex flex-col gap-3 border-b border-border p-3">
          <h1 className="font-display text-lg font-semibold tracking-tight">{t.memory.title}</h1>
          <ProjectPicker />
          <div className="flex gap-2">
            <Button
              variant="primary"
              onClick={() => setCreating(true)}
              disabled={projectId === null}
            >
              <Plus aria-hidden="true" />
              {t.memory.newMemory}
            </Button>
            <Button
              onClick={() => projectId !== null && importAdrs.mutate(projectId)}
              disabled={projectId === null || importAdrs.isPending}
            >
              <FileUp aria-hidden="true" />
              {importAdrs.isPending ? t.memory.importing : t.memory.importAdrs}
            </Button>
          </div>
          <Button onClick={() => setExporting(true)} disabled={projectId === null}>
            <FolderUp aria-hidden="true" />
            {t.memory.share.button}
          </Button>
          {projectId !== null && (
            <MemoryExportDialog
              projectId={projectId}
              open={exporting}
              onOpenChange={setExporting}
            />
          )}
          {report && (
            <div role="status" className="text-xs text-fg-muted">
              <p>
                {t.memory.importResult(
                  report.created,
                  report.updated,
                  report.superseding,
                  report.unchanged,
                )}
              </p>
              {report.skippedStale.length > 0 && (
                <p className="text-warning">
                  {t.memory.importStale(report.skippedStale.join(", "))}
                </p>
              )}
            </div>
          )}
          {importAdrs.isError && <ErrorState error={importAdrs.error} />}
          <input
            type="search"
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder={t.memory.searchPlaceholder}
            aria-label={t.memory.searchPlaceholder}
            className="h-9 rounded-lg border border-border bg-surface-1 shadow-card px-2.5 text-sm text-fg placeholder:text-fg-subtle"
          />
          <select
            value={kind}
            onChange={(e) => setKind(e.target.value as MemoryKind | "all")}
            aria-label={t.memory.editor.kind}
            className="h-9 rounded-lg border border-border bg-surface-1 shadow-card px-2.5 text-sm text-fg"
          >
            <option value="all">{t.memory.allKinds}</option>
            {MEMORY_KINDS.map((k) => (
              <option key={k} value={k}>
                {t.memoryKinds[k]}
              </option>
            ))}
          </select>
          <div className="flex flex-wrap gap-1" role="tablist" aria-label={t.memory.listLabel}>
            {TABS.map((s) => (
              <button
                key={s}
                type="button"
                role="tab"
                aria-selected={tab === s}
                onClick={() => setTab(s)}
                className={cn(
                  "rounded-md px-2 py-1 text-xs",
                  tab === s ? "bg-accent-subtle text-fg" : "text-fg-muted hover:bg-surface-2",
                )}
              >
                {t.memory.tabs[s]} ({counts.data?.[s] ?? 0})
              </button>
            ))}
          </div>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
          {list.isError && <ErrorState error={list.error} />}
          {list.data?.length === 0 && (
            <p className="px-2 py-1 text-sm text-fg-muted">{t.memory.empty}</p>
          )}
          <ul className="flex flex-col gap-0.5" aria-label={t.memory.listLabel}>
            {list.data?.map((m) => (
              <li key={m.id}>
                <button
                  type="button"
                  onClick={() => selectMemory(m.id)}
                  aria-current={m.id === selectedId ? "true" : undefined}
                  className={cn(
                    "flex w-full flex-col gap-0.5 rounded-md px-2.5 py-2 text-left",
                    m.id === selectedId ? "bg-accent-subtle" : "hover:bg-surface-2",
                  )}
                >
                  <span className="line-clamp-2 text-sm text-fg">{m.title}</span>
                  <span className="flex items-center gap-1.5 text-xs text-fg-muted">
                    <span>{t.memoryKinds[m.kind]}</span>
                    {m.originPath && <Badge>{t.memory.imported}</Badge>}
                    <span title={formatDateTime(m.updatedAt)}>{formatRelative(m.updatedAt)}</span>
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </div>
      </aside>
      <div className="min-w-0 flex-1 overflow-y-auto">
        {selectedId === null ? (
          <div className="flex h-full items-center justify-center">
            <EmptyState icon={Brain} title={t.memory.emptyTitle} description={t.memory.emptyBody} />
          </div>
        ) : (
          <MemoryView key={selectedId} memoryId={selectedId} />
        )}
      </div>
      {projectId !== null && (
        <MemoryEditor
          open={creating}
          onOpenChange={setCreating}
          projectId={projectId}
          onSaved={(m) => {
            setTab(m.status);
            selectMemory(m.id);
          }}
        />
      )}
    </div>
  );
}

function MemoryView({ memoryId }: { memoryId: string }) {
  const detail = useMemory(memoryId);
  if (detail.isError) {
    return (
      <div className="p-6">
        <ErrorState error={detail.error} onRetry={() => void detail.refetch()} />
      </div>
    );
  }
  if (!detail.data) {
    return (
      <div className="flex flex-col gap-3 p-6" aria-busy="true">
        <Skeleton className="h-8 w-2/3" />
        <Skeleton className="h-40" />
      </div>
    );
  }
  return <MemoryContent detail={detail.data} />;
}

function MemoryContent({ detail }: { detail: MemoryDetail }) {
  const m = detail.memory;
  const selectMemory = useUi((s) => s.selectMemory);
  const [editing, setEditing] = useState(false);
  const { transition, reviewRefs } = useMemoryMutations();
  const alerts = useFreshnessIssues({ projectId: m.projectId, memoryId: m.id });
  const busy = transition.isPending || reviewRefs.isPending;
  const act = (to: "confirm" | "archive" | "restore") =>
    transition.mutate({
      memoryId: m.id,
      transition: to,
      expectedRevision: m.revision,
      note: null,
    });
  const refsNeedReview = detail.refChecks.some((r) => r.state === "changed" || r.state === "moved");
  const active = m.status === "proposed" || m.status === "confirmed";

  return (
    <div className="mx-auto flex w-full max-w-[1000px] flex-col gap-4 px-6 py-5">
      <header className="flex flex-col gap-2">
        <div className="flex flex-wrap items-center gap-2">
          <Badge tone="accent">{t.memoryKinds[m.kind]}</Badge>
          <Badge tone={statusTone[m.status]}>{t.memoryStatus[m.status]}</Badge>
          <Badge>{t.memory.origin[m.origin] ?? m.origin}</Badge>
        </div>
        <h2 className="text-xl font-semibold tracking-tight">{m.title}</h2>
        <p className="text-xs text-fg-muted">
          {t.memory.revision(m.revision)} ·{" "}
          <span title={formatDateTime(m.updatedAt)}>
            {t.memory.updated(formatRelative(m.updatedAt))}
          </span>
          {m.originPath && <> · {t.memory.importedFrom(m.originPath)}</>}
        </p>
        {detail.supersedesTitle && m.supersedes && (
          <p className="text-xs text-fg-muted">
            {t.memory.supersedes}:{" "}
            <button
              type="button"
              className="text-accent-text underline-offset-2 hover:underline"
              onClick={() => selectMemory(m.supersedes)}
            >
              {detail.supersedesTitle}
            </button>
          </p>
        )}
        {detail.supersededByTitle && m.supersededBy && (
          <p className="text-xs text-fg-muted">
            {t.memory.supersededBy}:{" "}
            <button
              type="button"
              className="text-accent-text underline-offset-2 hover:underline"
              onClick={() => selectMemory(m.supersededBy)}
            >
              {detail.supersededByTitle}
            </button>
          </p>
        )}
        <div className="flex flex-wrap gap-2">
          {m.status === "proposed" && (
            <Button variant="primary" onClick={() => act("confirm")} disabled={busy}>
              <CheckCheck aria-hidden="true" />
              {t.memory.confirm}
            </Button>
          )}
          {active && (
            <Button onClick={() => setEditing(true)} disabled={busy}>
              <Pencil aria-hidden="true" />
              {t.memory.edit}
            </Button>
          )}
          {active && refsNeedReview && (
            <Tooltip content={t.memory.reviewRefsHint}>
              <Button
                onClick={() => reviewRefs.mutate({ memoryId: m.id, expectedRevision: m.revision })}
                disabled={busy}
              >
                <RefreshCcw aria-hidden="true" />
                {t.memory.reviewRefs}
              </Button>
            </Tooltip>
          )}
          {active && (
            <Button variant="ghost" onClick={() => act("archive")} disabled={busy}>
              <Archive aria-hidden="true" />
              {t.memory.archive}
            </Button>
          )}
          {m.status === "archived" && (
            <Button onClick={() => act("restore")} disabled={busy}>
              <ArchiveRestore aria-hidden="true" />
              {t.memory.restore}
            </Button>
          )}
        </div>
        {(transition.error ?? reviewRefs.error) && (
          <ErrorState error={transition.error ?? reviewRefs.error} />
        )}
      </header>

      {m.status === "proposed" && (
        <p
          role="note"
          className="flex gap-2 rounded-md border border-warning/40 bg-warning-subtle px-3 py-2 text-sm"
        >
          <TriangleAlert className="mt-0.5 size-4 shrink-0 text-warning" aria-hidden="true" />
          {detail.supersedesTitle
            ? t.memory.supersedingNotice(detail.supersedesTitle)
            : t.memory.proposalNotice}
        </p>
      )}

      {m.body && (
        <Panel>
          <p className="selectable text-sm whitespace-pre-wrap text-fg">{m.body}</p>
        </Panel>
      )}

      <Panel title={t.memory.references}>
        {detail.refChecks.length === 0 ? (
          <p className="text-sm text-fg-muted">{t.memory.noRefs}</p>
        ) : (
          <ul className="flex flex-col gap-1.5">
            {detail.refChecks.map((r) => (
              <li
                key={`${r.reference.relPath}:${r.reference.startLine ?? ""}`}
                className="flex flex-wrap items-center gap-2"
              >
                <span className="font-mono text-sm">
                  {r.reference.relPath}
                  {r.reference.startLine !== null &&
                    r.reference.endLine !== null &&
                    `:${r.reference.startLine}-${r.reference.endLine}`}
                </span>
                <Tooltip content={t.memory.refStateHints[r.state]}>
                  <Badge tone={refTone[r.state]}>{t.memory.refStates[r.state]}</Badge>
                </Tooltip>
                {r.movedTo && (
                  <span className="font-mono text-xs text-fg-muted">
                    {t.memory.movedTo(r.movedTo)}
                  </span>
                )}
              </li>
            ))}
          </ul>
        )}
      </Panel>

      <Panel title={t.memory.alerts}>
        {alerts.data?.length === 0 && <p className="text-sm text-fg-muted">{t.memory.noAlerts}</p>}
        <div className="flex flex-col gap-3">
          {alerts.data?.map((issue) => (
            <IssueCard key={issue.id} issue={issue} compact />
          ))}
        </div>
      </Panel>

      <Panel title={t.memory.history}>
        <ol className="flex flex-col gap-2">
          {detail.revisions.map((r) => (
            <li key={r.revision} className="text-sm">
              <details>
                <summary className="cursor-pointer">
                  <span className="font-medium">rev {r.revision}</span> ·{" "}
                  {t.memory.changes[r.change] ?? r.change} · {t.memory.actors[r.actor] ?? r.actor} ·{" "}
                  <span className="text-fg-muted">{formatDateTime(r.changedAt)}</span>
                  {r.note && <span className="text-fg-muted"> — {r.note}</span>}
                </summary>
                <div className="mt-1.5 ml-4 rounded-md border border-border bg-surface-2 p-2">
                  <p className="text-xs text-fg-muted">
                    {t.memoryKinds[r.kind]} · {t.memoryStatus[r.status]}
                  </p>
                  <p className="font-medium">{r.title}</p>
                  {r.body && (
                    <p className="selectable mt-1 text-xs whitespace-pre-wrap">{r.body}</p>
                  )}
                </div>
              </details>
            </li>
          ))}
        </ol>
      </Panel>

      <MemoryEditor open={editing} onOpenChange={setEditing} projectId={m.projectId} memory={m} />
    </div>
  );
}
