import { ErrorState } from "@/components/feedback";
import type { Commit } from "@/lib/bindings";
import { formatDateTime, formatRelative } from "@/lib/format";
import { t } from "@/lib/i18n/pt-BR";
import { useFileHistory } from "@/lib/queries";

const s = t.analysis;

/** Commits em uma linha cada: hash curto, assunto, autor e quando. */
export function CommitList({ commits }: { commits: Commit[] }) {
  return (
    <ul className="flex flex-col gap-1">
      {commits.map((c) => (
        <li key={c.hash} className="flex gap-2 text-xs">
          <span className="shrink-0 font-mono text-fg-muted">{c.shortHash}</span>
          <span className="min-w-0 flex-1 truncate" title={c.subject}>
            {c.subject}
          </span>
          <span className="shrink-0 text-fg-muted" title={formatDateTime(c.dateMs)}>
            {c.author} · {formatRelative(c.dateMs)}
          </span>
        </li>
      ))}
    </ul>
  );
}

/** Últimos commits do arquivo na prévia da busca (git instalado, só leitura). */
export function FileHistory({ projectId, relPath }: { projectId: number; relPath: string }) {
  const history = useFileHistory(projectId, relPath);
  if (history.isError) {
    return (
      <section aria-label={s.history} className="mt-3 text-sm">
        <ErrorState error={history.error} onRetry={() => void history.refetch()} />
      </section>
    );
  }
  if (!history.data) return null;
  const { gitAvailable, commits } = history.data;
  return (
    <section aria-label={s.history} className="mt-3 text-sm">
      <h3 className="mb-1 text-xs font-medium text-fg-subtle">{s.historyTitle(commits.length)}</h3>
      {!gitAvailable ? (
        <p className="text-xs text-fg-muted">{s.historyNoGit}</p>
      ) : commits.length === 0 ? (
        <p className="text-xs text-fg-muted">{s.historyEmpty}</p>
      ) : (
        <CommitList commits={commits} />
      )}
    </section>
  );
}
