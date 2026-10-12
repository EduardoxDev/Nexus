import type { Edge } from "@/lib/bindings";
import { t } from "@/lib/i18n/pt-BR";
import { useFileAnalysis } from "@/lib/queries";

const s = t.analysis;

/** Declarações e dependências do arquivo na prévia da busca. */
export function FileDependencies({
  projectId,
  relPath,
  onOpen,
}: {
  projectId: number;
  relPath: string;
  onOpen: (path: string) => void;
}) {
  const view = useFileAnalysis(projectId, relPath);
  if (!view.data || !view.data.analyzed) return null;
  const v = view.data;
  const target = (e: Edge) =>
    e.toPath ?? (e.external ? s.externalPackage(e.external) : s.unresolved);
  return (
    <section aria-label={s.dependencies} className="mt-3 flex flex-col gap-3 text-sm">
      {v.symbols.length > 0 && (
        <div>
          <h3 className="mb-1 text-xs font-medium text-fg-subtle">
            {s.declarations(v.symbols.length)}
          </h3>
          <ul className="flex flex-wrap gap-x-3 gap-y-0.5">
            {v.symbols.slice(0, 30).map((sym) => (
              <li key={`${sym.name}:${sym.line}`} className="text-xs">
                <span className="font-mono">{sym.name}</span>{" "}
                <span className="text-fg-muted">
                  {t.symbolKinds[sym.kind]} · {sym.line}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
      <div>
        <h3 className="mb-1 text-xs font-medium text-fg-subtle">{s.imports(v.imports.length)}</h3>
        <ul aria-label={s.importsLabel} className="flex flex-col gap-0.5">
          {v.imports.map((e) => (
            <li key={`${e.line}:${e.specifier}`} className="flex gap-2 text-xs">
              <span className="w-8 shrink-0 text-right text-fg-muted tabular-nums">{e.line}</span>
              {e.toPath ? (
                <button
                  type="button"
                  onClick={() => onOpen(e.toPath ?? "")}
                  className="truncate font-mono text-accent-text hover:underline"
                >
                  {e.toPath}
                </button>
              ) : (
                <span className="truncate font-mono text-fg-muted">{target(e)}</span>
              )}
            </li>
          ))}
        </ul>
      </div>
      <div>
        <h3 className="mb-1 text-xs font-medium text-fg-subtle">
          {s.importedBy(v.importedBy.length)}
        </h3>
        <ul aria-label={s.importedByLabel} className="flex flex-col gap-0.5">
          {v.importedBy.map((e) => (
            <li key={`${e.fromPath}:${e.line}`} className="text-xs">
              <button
                type="button"
                onClick={() => onOpen(e.fromPath)}
                className="font-mono text-accent-text hover:underline"
              >
                {e.fromPath}
              </button>
              <span className="text-fg-muted"> :{e.line}</span>
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}
