import { Check, Copy, FileDown, FolderGit2, Package, Trash2, TriangleAlert } from "lucide-react";
import { useEffect, useMemo, useState, type SubmitEvent } from "react";

import { CodeBlock, FreshnessBadge } from "@/components/code";
import { EmptyState, ErrorState } from "@/components/feedback";
import { Panel } from "@/components/layout";
import { ProjectPicker } from "@/components/ProjectPicker";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import type { ContextPack, ExportFormat, SourceRole } from "@/lib/bindings";
import { cn } from "@/lib/cn";
import { formatDateTime, formatRelative, shortHash } from "@/lib/format";
import { t } from "@/lib/i18n/pt-BR";
import {
  useCompilePack,
  useContextPack,
  useContextPacks,
  useDeletePack,
  usePackExport,
  useProjects,
} from "@/lib/queries";
import { useUi } from "@/stores/ui";

const BUDGETS = [4000, 8000, 16000, 32000];
const ROLE_ORDER: SourceRole[] = [
  "primary",
  "test",
  "decision",
  "convention",
  "related",
  "doc",
  "dependency",
  "config",
];

export function ContextPacksPage() {
  const projects = useProjects();
  const projectId = useUi((s) => s.selectedProjectId);
  const navigate = useUi((s) => s.navigate);
  // Tarefa vinda da Busca: usada como valor inicial e consumida ao montar.
  const [task, setTask] = useState(() => useUi.getState().draftTask);
  const [budget, setBudget] = useState(8000);
  const [maxFiles, setMaxFiles] = useState(12);
  const [selection, setSelection] = useState<{ projectId: number; packId: string } | null>(null);
  const packId = selection !== null && selection.projectId === projectId ? selection.packId : null;
  const setPackId = (id: string | null) =>
    setSelection(id === null || projectId === null ? null : { projectId, packId: id });
  const packs = useContextPacks(projectId);
  const compile = useCompilePack();

  useEffect(() => {
    const ui = useUi.getState();
    if (ui.draftTask) ui.setDraftTask("");
  }, []);

  const onSubmit = (e: SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (projectId === null || task.trim().length < 3) return;
    compile.mutate(
      { projectId, task: task.trim(), tokenBudget: budget, maxFiles },
      { onSuccess: (pack) => setPackId(pack.plan.id) },
    );
  };

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

  return (
    <div className="flex h-full min-h-0">
      <aside className="flex w-80 shrink-0 flex-col border-r border-border bg-surface-1">
        <form onSubmit={onSubmit} className="flex flex-col gap-3 border-b border-border p-3">
          <div className="flex items-center justify-between">
            <h1 className="text-base font-semibold">{t.packs.newTitle}</h1>
          </div>
          <ProjectPicker />
          <label className="flex flex-col gap-1 text-xs text-fg-muted">
            {t.packs.taskLabel}
            <textarea
              value={task}
              onChange={(e) => setTask(e.target.value)}
              placeholder={t.packs.taskPlaceholder}
              maxLength={2000}
              rows={4}
              className="resize-y rounded-md border border-border bg-surface-2 p-2 text-sm text-fg placeholder:text-fg-subtle focus:border-accent focus:outline-none"
            />
          </label>
          <div className="grid grid-cols-2 gap-2">
            <label className="flex flex-col gap-1 text-xs text-fg-muted">
              {t.packs.budget}
              <select
                value={budget}
                onChange={(e) => setBudget(Number(e.target.value))}
                className="h-8 rounded-md border border-border bg-surface-2 px-2 text-sm text-fg"
              >
                {BUDGETS.map((b) => (
                  <option key={b} value={b}>
                    {b.toLocaleString("pt-BR")}
                  </option>
                ))}
              </select>
            </label>
            <label className="flex flex-col gap-1 text-xs text-fg-muted">
              {t.packs.maxFiles}
              <select
                value={maxFiles}
                onChange={(e) => setMaxFiles(Number(e.target.value))}
                className="h-8 rounded-md border border-border bg-surface-2 px-2 text-sm text-fg"
              >
                {[6, 12, 20, 30].map((n) => (
                  <option key={n} value={n}>
                    {n}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <Button
            type="submit"
            variant="primary"
            disabled={projectId === null || task.trim().length < 3 || compile.isPending}
          >
            <Package aria-hidden="true" />
            {compile.isPending ? t.packs.compiling : t.packs.compile}
          </Button>
          {compile.isError && <ErrorState error={compile.error} />}
        </form>
        <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
          <p className="px-2 pt-1 pb-1.5 text-xs font-medium text-fg-subtle">{t.packs.history}</p>
          {packs.data?.length === 0 && (
            <p className="px-2 text-sm text-fg-muted">{t.packs.historyEmpty}</p>
          )}
          <ul className="flex flex-col gap-0.5">
            {packs.data?.map((p) => (
              <li key={p.id}>
                <button
                  type="button"
                  onClick={() => setPackId(p.id)}
                  aria-current={p.id === packId ? "true" : undefined}
                  className={cn(
                    "flex w-full flex-col rounded-md px-2.5 py-2 text-left",
                    p.id === packId ? "bg-accent-subtle" : "hover:bg-surface-2",
                  )}
                >
                  <span className="line-clamp-2 text-sm text-fg">{p.task}</span>
                  <span className="text-xs text-fg-muted" title={formatDateTime(p.createdAt)}>
                    {formatRelative(p.createdAt)} · {p.fileCount} fontes
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </div>
      </aside>
      <div className="min-w-0 flex-1 overflow-y-auto">
        {packId === null ? (
          <div className="flex h-full items-center justify-center">
            <EmptyState icon={Package} title={t.packs.emptyTitle} description={t.packs.emptyBody} />
          </div>
        ) : (
          <PackView key={packId} packId={packId} onDeleted={() => setPackId(null)} />
        )}
      </div>
    </div>
  );
}

function PackView({ packId, onDeleted }: { packId: string; onDeleted: () => void }) {
  const pack = useContextPack(packId);
  if (pack.isError) {
    return (
      <div className="p-6">
        <ErrorState error={pack.error} onRetry={() => void pack.refetch()} />
      </div>
    );
  }
  if (!pack.data) {
    return (
      <div className="flex flex-col gap-3 p-6" aria-busy="true">
        <Skeleton className="h-8 w-2/3" />
        <Skeleton className="h-40" />
      </div>
    );
  }
  return <PackContent pack={pack.data} onDeleted={onDeleted} />;
}

function PackContent({ pack, onDeleted }: { pack: ContextPack; onDeleted: () => void }) {
  const { plan } = pack;
  const [excluded, setExcluded] = useState<string[]>([]);
  const [notes, setNotes] = useState("");
  const [copied, setCopied] = useState(false);
  const [savedTo, setSavedTo] = useState<string | null>(null);
  const exporter = usePackExport();
  const remove = useDeletePack();
  const options = useMemo(
    () => ({ excludedPaths: excluded, ...(notes.trim() ? { notes } : {}) }),
    [excluded, notes],
  );

  const items = plan.sources.map((source, i) => ({ source, excerpt: pack.excerpts[i] }));
  const byRole = ROLE_ORDER.map((role) => ({
    role,
    items: items.filter((it) => it.source.role === role),
  })).filter((g) => g.items.length > 0);
  const includedTokens = plan.sources
    .filter((s) => !excluded.includes(s.relPath))
    .reduce((sum, s) => sum + s.estimatedTokens, 0);

  const copyMarkdown = () => {
    exporter.render.mutate(
      { packId: plan.id, format: "markdown", options },
      {
        onSuccess: (text) => {
          void navigator.clipboard.writeText(text).then(() => {
            setCopied(true);
            window.setTimeout(() => setCopied(false), 1500);
          });
        },
      },
    );
  };
  const save = (format: ExportFormat) =>
    exporter.save.mutate(
      { packId: plan.id, format, options },
      { onSuccess: (path) => setSavedTo(path) },
    );

  return (
    <div className="mx-auto flex w-full max-w-[1000px] flex-col gap-4 px-6 py-5">
      <header className="flex flex-col gap-2">
        <h2 className="text-xl font-semibold tracking-tight">{plan.task}</h2>
        <p className="text-xs text-fg-muted">
          {formatDateTime(plan.createdAt)} · {plan.projectName} ·{" "}
          {t.packs.meta(
            plan.indexGeneration,
            plan.headCommit ? shortHash(plan.headCommit, 8) : t.packs.noGit,
          )}{" "}
          · {t.packs.tokens(includedTokens, plan.tokenBudget)}
        </p>
        <div className="flex flex-wrap gap-2">
          <Button variant="primary" onClick={copyMarkdown} disabled={exporter.render.isPending}>
            {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
            {copied ? t.packs.copied : t.packs.copyMd}
          </Button>
          <Button onClick={() => save("markdown")} disabled={exporter.save.isPending}>
            <FileDown aria-hidden="true" />
            {t.packs.saveMd}
          </Button>
          <Button onClick={() => save("json")} disabled={exporter.save.isPending}>
            <FileDown aria-hidden="true" />
            {t.packs.saveJson}
          </Button>
          <Button
            variant="ghost"
            onClick={() => remove.mutate(plan.id, { onSuccess: onDeleted })}
            disabled={remove.isPending}
          >
            <Trash2 aria-hidden="true" />
            {t.packs.delete}
          </Button>
        </div>
        {savedTo && (
          <p className="text-xs text-success" role="status">
            {t.packs.saved(savedTo)}
          </p>
        )}
        {(exporter.render.error ?? exporter.save.error ?? remove.error) && (
          <ErrorState error={exporter.render.error ?? exporter.save.error ?? remove.error} />
        )}
        <p className="text-xs text-fg-subtle">{t.packs.untrustedNotice}</p>
      </header>

      {pack.staleSources > 0 && (
        <p
          role="alert"
          className="flex gap-2 rounded-md border border-warning/40 bg-warning-subtle px-3 py-2 text-sm"
        >
          <TriangleAlert className="mt-0.5 size-4 shrink-0 text-warning" aria-hidden="true" />
          {t.packs.stale(pack.staleSources)}
        </p>
      )}

      <Panel title={t.packs.summary}>
        <p className="text-sm text-fg">{plan.summary}</p>
      </Panel>

      {byRole.map((group) => (
        <Panel
          key={group.role}
          title={t.packs.roleTitles[group.role]}
          bodyClassName="flex flex-col gap-4"
        >
          {group.items.map(({ source, excerpt }) => {
            const included = !excluded.includes(source.relPath);
            return (
              <div key={source.relPath} className={cn(!included && "opacity-50")}>
                <div className="mb-1.5 flex flex-wrap items-center gap-2">
                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={included}
                      onChange={() =>
                        setExcluded((cur) =>
                          included
                            ? [...cur, source.relPath]
                            : cur.filter((p) => p !== source.relPath),
                        )
                      }
                      aria-label={`${t.packs.include}: ${source.relPath}`}
                      className="size-4 accent-[var(--accent)]"
                    />
                    <span className="font-mono text-sm">{source.relPath}</span>
                  </label>
                  {excerpt && (
                    <>
                      <Badge>{t.packs.excerptLines(excerpt.startLine, excerpt.endLine)}</Badge>
                      <FreshnessBadge freshness={excerpt.freshness} />
                    </>
                  )}
                  <span className="ml-auto text-xs text-fg-subtle">
                    ~{source.estimatedTokens} tokens
                  </span>
                </div>
                {source.reasons.length > 0 && (
                  <p className="mb-1.5 text-xs text-fg-muted">{source.reasons.join(" · ")}</p>
                )}
                {excerpt && excerpt.text && (
                  <CodeBlock
                    text={excerpt.text}
                    startLine={excerpt.startLine}
                    className="max-h-80 overflow-y-auto"
                    label={source.relPath}
                  />
                )}
              </div>
            );
          })}
        </Panel>
      ))}

      <Panel title={t.packs.risks}>
        {plan.risks.length === 0 ? (
          <p className="text-sm text-fg-muted">{t.packs.noRisks}</p>
        ) : (
          <ul className="list-disc pl-5 text-sm">
            {plan.risks.map((r) => (
              <li key={r}>{r}</li>
            ))}
          </ul>
        )}
      </Panel>
      <Panel title={t.packs.contradictions}>
        <p className="text-sm text-fg-muted">{t.packs.contradictionsPending}</p>
      </Panel>
      <Panel title={t.packs.limitations}>
        <ul className="list-disc pl-5 text-sm text-fg-muted">
          {plan.limitations.map((l) => (
            <li key={l}>{l}</li>
          ))}
        </ul>
        {plan.omitted.length > 0 && (
          <p className="mt-2 text-xs text-fg-muted">
            {t.packs.omitted}: <span className="font-mono">{plan.omitted.join(", ")}</span>
          </p>
        )}
      </Panel>
      <Panel title={t.packs.notes}>
        <textarea
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          placeholder={t.packs.notesPlaceholder}
          maxLength={10000}
          rows={3}
          aria-label={t.packs.notes}
          className="w-full resize-y rounded-md border border-border bg-surface-2 p-2 text-sm text-fg placeholder:text-fg-subtle focus:border-accent focus:outline-none"
        />
      </Panel>
    </div>
  );
}
