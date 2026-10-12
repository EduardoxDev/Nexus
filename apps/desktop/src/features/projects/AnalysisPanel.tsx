import { ErrorState } from "@/components/feedback";
import { Panel } from "@/components/layout";
import { Skeleton } from "@/components/ui/skeleton";
import { formatDuration, formatInteger } from "@/lib/format";
import { t } from "@/lib/i18n/pt-BR";
import { useAnalysisOverview } from "@/lib/queries";

const s = t.analysis;

/** Cobertura dos analisadores e estado do grafo de dependências do projeto. */
export function AnalysisPanel({ projectId }: { projectId: number }) {
  const overview = useAnalysisOverview(projectId);
  if (overview.isError) {
    return (
      <Panel title={s.title}>
        <ErrorState error={overview.error} />
      </Panel>
    );
  }
  const o = overview.data;
  return (
    <Panel
      title={s.title}
      description={
        o?.graph
          ? s.graphSummary(
              o.graph.edges,
              o.graph.resolved,
              o.graph.external,
              o.graph.unresolved,
              formatDuration(o.graph.durationMs),
            )
          : s.noGraph
      }
    >
      {!o ? (
        <Skeleton className="h-24" />
      ) : (
        <div className="flex flex-col gap-4">
          {o.languages.length === 0 ? (
            <p className="text-sm text-fg-muted">{s.noAnalyzed}</p>
          ) : (
            <table className="w-full text-sm" aria-label={s.title}>
              <thead>
                <tr className="text-left text-xs text-fg-subtle">
                  {s.columns.map((c) => (
                    <th key={c} className="pb-1 font-medium">
                      {c}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {o.languages.map((l) => (
                  <tr key={l.language} className="border-t border-border">
                    <td className="py-1">{l.language}</td>
                    <td className="tabular-nums">{formatInteger(l.files)}</td>
                    <td className="tabular-nums">{formatInteger(l.symbols)}</td>
                    <td className="tabular-nums">{formatInteger(l.resolved)}</td>
                    <td className="tabular-nums">{formatInteger(l.external)}</td>
                    <td className="tabular-nums">{formatInteger(l.unresolved)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          {o.notAnalyzed.length > 0 && (
            <p className="text-xs text-fg-muted">
              {s.notAnalyzed(o.notAnalyzed.map((c) => `${c.name} (${c.count})`).join(", "))}
            </p>
          )}
          {o.mostImported.length > 0 && (
            <div>
              <h3 className="mb-1 text-xs font-medium text-fg-subtle">{s.mostImported}</h3>
              <ul className="flex flex-col gap-0.5">
                {o.mostImported.slice(0, 6).map((c) => (
                  <li key={c.name} className="flex gap-2 text-sm">
                    <span className="w-8 text-right text-fg-muted tabular-nums">{c.count}</span>
                    <span className="font-mono text-xs">{c.name}</span>
                  </li>
                ))}
              </ul>
            </div>
          )}
          <details className="text-xs text-fg-muted">
            <summary className="cursor-pointer">{s.coverage}</summary>
            <ul className="mt-2 flex flex-col gap-2">
              {o.coverage.map((c) => (
                <li key={c.language}>
                  <p className="font-medium text-fg">{c.language}</p>
                  <p>
                    {s.covers}: {c.symbols.join("; ")} · {c.imports.join("; ")}
                  </p>
                  <p>
                    {s.notCovered}: {c.notCovered.join("; ")}
                  </p>
                </li>
              ))}
            </ul>
          </details>
        </div>
      )}
    </Panel>
  );
}
