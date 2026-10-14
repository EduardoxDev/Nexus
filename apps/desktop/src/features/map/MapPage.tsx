import { FolderGit2, Network } from "lucide-react";
import { useReducedMotionConfig } from "motion/react";
import { useState, type SubmitEvent } from "react";

import { EmptyState, ErrorState } from "@/components/feedback";
import { ProjectPicker } from "@/components/ProjectPicker";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import type { MapNode, ProjectMap } from "@/lib/bindings";
import { cn } from "@/lib/cn";
import { formatInteger } from "@/lib/format";
import { t } from "@/lib/i18n/pt-BR";
import { useProjectMap, useProjects } from "@/lib/queries";
import { useUi } from "@/stores/ui";
import { LANGUAGE_COLORS, MapCanvas, languageColor } from "./MapCanvas";

const s = t.map;
/** Linhas da tabela alternativa ao canvas. */
const TABLE_ROWS = 30;

export function MapPage() {
  const projects = useProjects();
  const projectId = useUi((st) => st.selectedProjectId);
  const navigate = useUi((st) => st.navigate);
  const openFile = useUi((st) => st.openFile);
  const [grouped, setGrouped] = useState(false);
  const [folder, setFolder] = useState("");
  const [folderDraft, setFolderDraft] = useState("");
  // Mesma preferência que o resto do app usa (MotionConfig no AppShell), reativa.
  const reducedMotion = useReducedMotionConfig() === true;
  const map = useProjectMap(
    projectId === null
      ? null
      : { projectId, groupByFolder: grouped, ...(folder ? { folder } : {}) },
  );
  const [hover, setHover] = useState<MapNode | null>(null);
  // Mapa novo (ou outro do cache): o nó apontado era do anterior, então é descartado.
  const [hoverMap, setHoverMap] = useState(map.data);
  if (hoverMap !== map.data) {
    setHoverMap(map.data);
    setHover(null);
  }

  if (projects.isError) {
    return (
      <div className="p-6">
        <ErrorState error={projects.error} onRetry={() => void projects.refetch()} />
      </div>
    );
  }

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

  const applyFolder = (value: string) => {
    setFolderDraft(value);
    setFolder(value.trim().replace(/^\/+|\/+$/g, ""));
  };
  const onSubmit = (e: SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    applyFolder(folderDraft);
  };
  /** Arquivo abre na Busca; pasta (modo agrupado) vira filtro e mostra os arquivos dela. */
  const open = (node: MapNode) => {
    if (map.data?.grouped) {
      setGrouped(false);
      // A pasta raiz (".") filtra só os arquivos que estão direto nela.
      applyFolder(node.id);
    } else {
      openFile(node.id);
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="border-b border-border px-6 pt-5 pb-4">
        <div className="flex items-start justify-between gap-4">
          <div>
            <h1 className="font-display text-2xl font-semibold tracking-tight">
              {t.nav.map.label}
            </h1>
            <p className="mt-0.5 text-sm text-fg-muted">{t.nav.map.description}</p>
          </div>
          <ProjectPicker />
        </div>
        <div className="mt-4 flex flex-wrap items-center gap-2">
          <form onSubmit={onSubmit} role="search" className="flex items-center gap-2">
            <input
              type="search"
              value={folderDraft}
              onChange={(e) => setFolderDraft(e.target.value)}
              placeholder={s.folderPlaceholder}
              aria-label={s.folderLabel}
              maxLength={300}
              className="h-9 w-64 rounded-lg border border-border bg-surface-1 px-3 font-mono text-sm text-fg shadow-card placeholder:font-sans placeholder:text-fg-subtle"
            />
            <Button type="submit" size="sm">
              {s.applyFolder}
            </Button>
            {folder && (
              <Button type="button" size="sm" variant="ghost" onClick={() => applyFolder("")}>
                {s.clearFolder}
              </Button>
            )}
          </form>
          <button
            type="button"
            aria-pressed={grouped}
            onClick={() => setGrouped((g) => !g)}
            className={cn(
              "h-9 rounded-lg border px-3 text-sm transition-colors",
              grouped
                ? "border-accent bg-accent-subtle text-accent-text"
                : "border-border text-fg-muted hover:text-fg",
            )}
          >
            {s.groupByFolder}
          </button>
          {map.data && (
            <p className="ml-auto text-xs text-fg-muted">
              {s.showing(map.data.nodes.length, map.data.totalNodes, map.data.grouped)}
            </p>
          )}
        </div>
      </div>

      {map.isError ? (
        <div className="p-6">
          <ErrorState error={map.error} onRetry={() => void map.refetch()} />
        </div>
      ) : !map.data ? (
        <div className="flex-1 p-6" aria-busy="true">
          <Skeleton className="h-full" />
        </div>
      ) : map.data.nodes.length === 0 ? (
        <div className="flex flex-1 items-center justify-center">
          <EmptyState
            icon={Network}
            title={folder ? s.emptyFolder(folder) : s.emptyTitle}
            description={s.emptyDescription}
          />
        </div>
      ) : (
        <div className="flex min-h-0 flex-1">
          <div className="dot-grid relative min-w-0 flex-1">
            <MapCanvas
              map={map.data}
              reducedMotion={reducedMotion}
              onOpen={open}
              onHover={(node) => {
                setHover(node);
              }}
            />
            <Legend map={map.data} />
            {hover && <HoverCard node={hover} grouped={map.data.grouped} />}
            <p className="pointer-events-none absolute right-3 bottom-3 text-xs text-fg-subtle">
              {s.hint}
            </p>
          </div>
          <CentralTable map={map.data} onOpen={open} />
        </div>
      )}
    </div>
  );
}

function Legend({ map }: { map: ProjectMap }) {
  const present = new Set(map.nodes.map((n) => n.language ?? ""));
  const items = Object.keys(LANGUAGE_COLORS).filter((l) => present.has(l));
  const other = [...present].some((l) => !(l in LANGUAGE_COLORS));
  return (
    <ul
      aria-label={s.legend}
      className="absolute top-3 left-3 flex flex-wrap gap-x-3 gap-y-1 rounded-lg border border-border bg-surface-1/90 px-3 py-2 text-xs text-fg-muted shadow-card backdrop-blur"
    >
      {items.map((l) => (
        <li key={l} className="flex items-center gap-1.5">
          <span
            aria-hidden="true"
            className="size-2.5 rounded-full"
            style={{ background: languageColor(l) }}
          />
          {l}
        </li>
      ))}
      {other && (
        <li className="flex items-center gap-1.5">
          <span
            aria-hidden="true"
            className="size-2.5 rounded-full"
            style={{ background: languageColor(null) }}
          />
          {s.otherLanguages}
        </li>
      )}
    </ul>
  );
}

function HoverCard({ node, grouped }: { node: MapNode; grouped: boolean }) {
  return (
    <div
      role="status"
      className="pointer-events-none absolute top-3 right-3 max-w-sm rounded-lg border border-border bg-surface-1 px-3 py-2 text-xs shadow-card"
    >
      <p className="font-mono text-sm break-all text-fg">{node.id}</p>
      <p className="mt-1 text-fg-muted">
        {grouped ? s.folderFiles(node.files) : (node.language ?? s.noLanguage)} ·{" "}
        {s.importedBy(node.inDegree)} · {s.imports(node.outDegree)}
      </p>
      <p className="mt-1 text-fg-subtle">{grouped ? s.clickFolder : s.clickFile}</p>
    </div>
  );
}

/** Alternativa acessível ao canvas: os nós mais centrais, com os mesmos números. */
function CentralTable({ map, onOpen }: { map: ProjectMap; onOpen: (n: MapNode) => void }) {
  return (
    <aside
      aria-label={s.tableTitle}
      className="flex w-[340px] shrink-0 flex-col border-l border-border bg-surface-1"
    >
      <h2 className="border-b border-border px-4 py-2.5 text-sm font-medium">{s.tableTitle}</h2>
      <div className="min-h-0 flex-1 overflow-y-auto">
        <table className="w-full text-xs">
          <thead className="sticky top-0 bg-surface-1 text-fg-subtle">
            <tr>
              <th scope="col" className="px-4 py-1.5 text-left font-medium">
                {map.grouped ? s.colFolder : s.colFile}
              </th>
              <th scope="col" className="px-2 py-1.5 text-right font-medium whitespace-nowrap">
                {s.colIn}
              </th>
              <th scope="col" className="px-4 py-1.5 text-right font-medium whitespace-nowrap">
                {s.colOut}
              </th>
            </tr>
          </thead>
          <tbody>
            {map.nodes.slice(0, TABLE_ROWS).map((n) => (
              <tr key={n.id} className="border-t border-border/60">
                <td className="max-w-0 px-4 py-1.5">
                  <button
                    type="button"
                    onClick={() => onOpen(n)}
                    title={n.id}
                    aria-label={n.id}
                    className="group flex w-full items-center gap-2 text-left"
                  >
                    <span
                      aria-hidden="true"
                      className="size-2 shrink-0 rounded-full"
                      style={{ background: languageColor(n.language) }}
                    />
                    <span className="min-w-0">
                      <span className="block truncate font-mono text-fg group-hover:text-accent-text">
                        {map.grouped ? n.id : (n.id.split("/").at(-1) ?? n.id)}
                      </span>
                      {!map.grouped && (
                        <span className="block truncate text-[11px] text-fg-subtle">
                          {n.folder}
                        </span>
                      )}
                    </span>
                  </button>
                </td>
                <td className="px-2 py-1.5 text-right tabular-nums">{formatInteger(n.inDegree)}</td>
                <td className="px-4 py-1.5 text-right tabular-nums">
                  {formatInteger(n.outDegree)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </aside>
  );
}
