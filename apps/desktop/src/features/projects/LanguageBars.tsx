import type { LanguageStat } from "@/lib/bindings";
import { formatBytes, formatInteger } from "@/lib/format";
import { t } from "@/lib/i18n/pt-BR";

/**
 * Comparação de magnitude: barras horizontais de uma única cor, ordenadas por tamanho, com os
 * valores sempre escritos em texto (a cor nunca é a única fonte da informação).
 * Sem paleta categórica: as linguagens são rotuladas, não coloridas.
 */
export function LanguageBars({ languages }: { languages: LanguageStat[] }) {
  if (languages.length === 0) {
    return <p className="text-sm text-fg-muted">{t.projects.languagesEmpty}</p>;
  }
  const max = Math.max(...languages.map((l) => l.bytes), 1);
  return (
    <ul aria-label={t.projects.languages} className="flex flex-col gap-2.5">
      {languages.map((l) => {
        const detail = t.projects.languageRow(formatInteger(l.files), formatInteger(l.lines));
        return (
          <li
            key={l.language}
            className="group"
            title={`${l.language}: ${formatBytes(l.bytes)} · ${detail}`}
          >
            <div className="flex items-baseline justify-between gap-3">
              <span className="truncate text-sm text-fg">{l.language}</span>
              <span className="shrink-0 text-xs text-fg-muted">
                <span className="font-mono">{formatBytes(l.bytes)}</span> · {detail}
              </span>
            </div>
            <div className="mt-1 h-2 w-full" aria-hidden="true">
              <div
                className="h-2 rounded-r-[4px] bg-accent transition-opacity group-hover:opacity-80"
                style={{ width: `${Math.max(1, (l.bytes / max) * 100)}%` }}
              />
            </div>
          </li>
        );
      })}
    </ul>
  );
}
