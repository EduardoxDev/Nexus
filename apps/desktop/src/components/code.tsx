import { Badge } from "@/components/ui/badge";
import { Tooltip } from "@/components/ui/tooltip";
import type { Freshness } from "@/lib/bindings";
import { cn } from "@/lib/cn";
import { t } from "@/lib/i18n/pt-BR";

interface CodeBlockProps {
  text: string;
  startLine: number;
  /** Linhas destacadas (números absolutos). */
  highlight?: number[];
  className?: string;
  label?: string;
}

/** Trecho de código com números de linha. O texto é exibido como dado (nunca interpretado). */
export function CodeBlock({ text, startLine, highlight = [], className, label }: CodeBlockProps) {
  const lines = text.split("\n");
  const marked = new Set(highlight);
  return (
    <pre
      aria-label={label}
      className={cn(
        "selectable overflow-x-auto rounded-md border border-border bg-surface-2 py-2 font-mono text-xs leading-5",
        className,
      )}
    >
      <code>
        {lines.map((line, i) => {
          const n = startLine + i;
          const hit = marked.has(n);
          return (
            <div
              key={n}
              data-line={n}
              data-match={hit || undefined}
              className={cn("flex pr-3", hit && "bg-accent-subtle")}
            >
              <span
                aria-hidden="true"
                className={cn(
                  "w-12 shrink-0 pr-3 text-right text-fg-subtle select-none",
                  hit && "text-accent-text",
                )}
              >
                {n}
              </span>
              <span className="whitespace-pre text-fg">{line || " "}</span>
            </div>
          );
        })}
      </code>
    </pre>
  );
}

const tone = { fresh: "success", modified: "warning", missing: "danger" } as const;

export function FreshnessBadge({ freshness }: { freshness: Freshness }) {
  const tooltip = {
    fresh: t.freshness.freshTooltip,
    modified: t.freshness.modifiedTooltip,
    missing: t.freshness.missingTooltip,
  }[freshness];
  return (
    <Tooltip content={tooltip}>
      <span tabIndex={0}>
        <Badge tone={tone[freshness]}>{t.freshness[freshness]}</Badge>
      </span>
    </Tooltip>
  );
}
