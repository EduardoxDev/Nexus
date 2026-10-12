import { FolderGit2, Package, Search as SearchIcon, SearchX } from "lucide-react";
import { useState, type SubmitEvent } from "react";

import { CodeBlock, FreshnessBadge } from "@/components/code";
import { FileDependencies } from "@/features/search/FileDependencies";
import { EmptyState, ErrorState } from "@/components/feedback";
import { ProjectPicker } from "@/components/ProjectPicker";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Tooltip } from "@/components/ui/tooltip";
import type { FileKind, SearchHit, SearchRequest, SearchResponse } from "@/lib/bindings";
import { cn } from "@/lib/cn";
import { t } from "@/lib/i18n/pt-BR";
import { useFileContext, useProjects, useSearch } from "@/lib/queries";
import { useUi } from "@/stores/ui";

const KIND_FILTERS: FileKind[] = ["code", "test", "doc", "config"];

export function SearchPage() {
  const projects = useProjects();
  const projectId = useUi((s) => s.selectedProjectId);
  const navigate = useUi((s) => s.navigate);
  const setDraftTask = useUi((s) => s.setDraftTask);
  const [text, setText] = useState("");
  const [kinds, setKinds] = useState<FileKind[]>([]);
  const [request, setRequest] = useState<SearchRequest | null>(null);
  const [selected, setSelected] = useState<SearchHit | null>(null);
  const search = useSearch(request);

  const submit = (query: string) => {
    if (projectId === null || query.trim() === "") return;
    setSelected(null);
    setRequest({
      projectId,
      query: query.trim(),
      limit: 20,
      ...(kinds.length > 0 ? { kinds } : {}),
    });
  };
  const onSubmit = (e: SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    submit(text);
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
      <div className="flex min-w-0 flex-1 flex-col">
        <div className="border-b border-border px-6 pt-5 pb-4">
          <div className="flex items-start justify-between gap-4">
            <div>
              <h1 className="font-display text-2xl font-semibold tracking-tight">
                {t.nav.search.label}
              </h1>
              <p className="mt-0.5 text-sm text-fg-muted">{t.nav.search.description}</p>
            </div>
            <ProjectPicker />
          </div>
          <form onSubmit={onSubmit} role="search" className="mt-4 flex gap-2">
            <div className="relative flex-1">
              <SearchIcon
                className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-fg-subtle"
                aria-hidden="true"
              />
              <input
                autoFocus
                type="search"
                value={text}
                onChange={(e) => setText(e.target.value)}
                placeholder={t.searchPage.placeholder}
                aria-label={t.searchPage.placeholder}
                maxLength={500}
                className="h-10 w-full rounded-lg border border-border bg-surface-1 pr-3 pl-9 text-sm text-fg shadow-card placeholder:text-fg-subtle"
              />
            </div>
            <Button
              type="submit"
              variant="primary"
              size="lg"
              disabled={projectId === null || search.isFetching}
            >
              {search.isFetching ? t.searchPage.searching : t.searchPage.submit}
            </Button>
          </form>
          <div
            className="mt-2.5 flex flex-wrap items-center gap-1.5"
            role="group"
            aria-label={t.searchPage.kinds}
          >
            <span className="mr-1 text-xs text-fg-subtle">{t.searchPage.kinds}:</span>
            {KIND_FILTERS.map((kind) => {
              const on = kinds.includes(kind);
              return (
                <button
                  key={kind}
                  type="button"
                  aria-pressed={on}
                  onClick={() =>
                    setKinds((cur) => (on ? cur.filter((k) => k !== kind) : [...cur, kind]))
                  }
                  className={cn(
                    "h-6 rounded-sm border px-2 text-xs transition-colors",
                    on
                      ? "border-accent bg-accent-subtle text-accent-text"
                      : "border-border text-fg-muted hover:text-fg",
                  )}
                >
                  {t.fileKinds[kind]}
                </button>
              );
            })}
          </div>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto px-6 py-4">
          {request === null ? (
            <Intro
              onPick={(q) => {
                setText(q);
                submit(q);
              }}
            />
          ) : search.isError ? (
            <ErrorState error={search.error} onRetry={() => void search.refetch()} />
          ) : !search.data ? (
            <div className="flex flex-col gap-3" aria-busy="true">
              <Skeleton className="h-24" />
              <Skeleton className="h-24" />
            </div>
          ) : (
            <Results
              response={search.data}
              selected={selected}
              onSelect={setSelected}
              onPack={() => {
                setDraftTask(search.data.query);
                navigate("contextPacks");
              }}
            />
          )}
        </div>
      </div>
      {selected && projectId !== null && (
        <Preview
          key={`${selected.relPath}:${selected.startLine}`}
          projectId={projectId}
          hit={selected}
          onClose={() => setSelected(null)}
        />
      )}
    </div>
  );
}

function Intro({ onPick }: { onPick: (q: string) => void }) {
  return (
    <div className="mx-auto mt-10 max-w-lg text-center">
      <p className="text-base font-semibold text-fg">{t.searchPage.introTitle}</p>
      <p className="mt-1 text-sm text-fg-muted">{t.searchPage.introBody}</p>
      <div className="mt-4 flex flex-wrap justify-center gap-2">
        {t.searchPage.examples.map((q) => (
          <Button key={q} size="sm" onClick={() => onPick(q)}>
            {q}
          </Button>
        ))}
      </div>
    </div>
  );
}

function Results({
  response,
  selected,
  onSelect,
  onPack,
}: {
  response: SearchResponse;
  selected: SearchHit | null;
  onSelect: (hit: SearchHit) => void;
  onPack: () => void;
}) {
  const expansions = response.analysis.concepts.filter((c) => c.variants.length > 0);
  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-2 text-xs text-fg-muted">
        <span>
          {t.searchPage.understood}:{" "}
          {response.analysis.concepts.map((c) => c.term).join(", ") || "—"}
          {expansions.length > 0 && (
            <span className="text-fg-subtle">
              {" "}
              ({t.searchPage.expansions}:{" "}
              {expansions.map((c) => `${c.term} → ${c.variants.slice(0, 4).join(", ")}`).join("; ")}
              )
            </span>
          )}
        </span>
        <span>
          {t.searchPage.resultCount(response.hits.length, response.candidates, response.tookMs)}
        </span>
      </div>
      {response.warnings.map((w) => (
        <p
          key={w}
          role="note"
          className="rounded-md border border-warning/40 bg-warning-subtle px-3 py-2 text-sm text-fg"
        >
          {w}
        </p>
      ))}
      {response.hits.length === 0 ? (
        <EmptyState icon={SearchX} title={t.searchPage.noResults} />
      ) : (
        <>
          <ol className="flex flex-col gap-3" aria-label="Resultados">
            {response.hits.map((hit) => (
              <li key={hit.relPath}>
                <HitCard
                  hit={hit}
                  selected={selected?.relPath === hit.relPath}
                  onSelect={() => onSelect(hit)}
                />
              </li>
            ))}
          </ol>
          <div>
            <Button onClick={onPack}>
              <Package aria-hidden="true" />
              {t.searchPage.useInPack}
            </Button>
          </div>
        </>
      )}
    </div>
  );
}

function HitCard({
  hit,
  selected,
  onSelect,
}: {
  hit: SearchHit;
  selected: boolean;
  onSelect: () => void;
}) {
  const reasons = hit.reasons.filter((r) => r.weight > 0);
  return (
    <article
      className={cn(
        "rounded-lg border bg-surface-1 transition-colors",
        selected ? "border-accent" : "border-border hover:border-border-strong",
      )}
    >
      <button
        type="button"
        onClick={onSelect}
        aria-pressed={selected}
        className="flex w-full flex-wrap items-center gap-2 px-3 pt-2.5 pb-2 text-left"
      >
        <span className="font-mono text-sm text-fg">{hit.relPath}</span>
        <Badge>{t.searchPage.lines(hit.startLine, hit.endLine)}</Badge>
        <FreshnessBadge freshness={hit.freshness} />
        <Tooltip content={t.searchPage.score}>
          <span className="ml-auto font-mono text-xs text-fg-subtle">{hit.score.toFixed(3)}</span>
        </Tooltip>
      </button>
      {reasons.length > 0 && (
        <ul className="flex flex-wrap gap-1 px-3 pb-2" aria-label="Motivos">
          {reasons.map((r) => (
            <li key={r.label}>
              <Badge
                tone={r.kind === "definition" || r.kind === "mentioned" ? "accent" : "neutral"}
              >
                {r.label}
              </Badge>
            </li>
          ))}
        </ul>
      )}
      {hit.snippet && (
        <CodeBlock
          text={hit.snippet}
          startLine={hit.startLine}
          highlight={hit.matchLines}
          className="mx-3 mb-3"
          label={`${hit.relPath} ${t.searchPage.lines(hit.startLine, hit.endLine)}`}
        />
      )}
      {hit.alsoIn.length > 0 && (
        <p className="px-3 pb-2.5 text-xs text-fg-muted">
          {t.searchPage.alsoIn}: <span className="font-mono">{hit.alsoIn.join(", ")}</span>
        </p>
      )}
    </article>
  );
}

function Preview({
  projectId,
  hit,
  onClose,
}: {
  projectId: number;
  hit: SearchHit;
  onClose: () => void;
}) {
  // Navegar por uma dependência mostra o outro arquivo desde o início.
  const [path, setPath] = useState(hit.relPath);
  const own = path === hit.relPath;
  const start = own ? Math.max(1, hit.startLine - 20) : 1;
  const ctx = useFileContext(projectId, path, start, own ? hit.endLine + 60 : 400);
  return (
    <aside
      aria-label={t.searchPage.previewTitle}
      className="flex w-[44%] min-w-[360px] shrink-0 flex-col border-l border-border bg-surface-1"
    >
      <div className="flex items-center gap-2 border-b border-border px-4 py-2.5">
        <span className="min-w-0 flex-1 truncate font-mono text-sm">{path}</span>
        {!own && (
          <Button size="sm" variant="ghost" onClick={() => setPath(hit.relPath)}>
            {t.analysis.backToResult}
          </Button>
        )}
        {ctx.data && <FreshnessBadge freshness={ctx.data.freshness} />}
        <Button size="sm" variant="ghost" onClick={onClose}>
          {t.common.close}
        </Button>
      </div>
      <div className="min-h-0 flex-1 overflow-auto p-3">
        {ctx.isError ? (
          <ErrorState error={ctx.error} />
        ) : !ctx.data ? (
          <Skeleton className="h-64" />
        ) : (
          <>
            <p className="mb-2 text-xs text-fg-muted">
              {t.searchPage.lines(ctx.data.startLine, ctx.data.endLine)} ·{" "}
              {t.searchPage.totalLines(ctx.data.totalLines)}
            </p>
            <CodeBlock
              text={ctx.data.text}
              startLine={ctx.data.startLine}
              highlight={own ? hit.matchLines : []}
              label={t.searchPage.previewTitle}
            />
            <FileDependencies projectId={projectId} relPath={path} onOpen={setPath} />
          </>
        )}
      </div>
    </aside>
  );
}
