import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import { cn } from "@/lib/cn";

interface PageProps {
  title: string;
  description: string;
  actions?: ReactNode;
  children: ReactNode;
}

/** Página padrão: título, descrição e conteúdo rolável. */
export function Page({ title, description, actions, children }: PageProps) {
  return (
    <div className="mx-auto flex w-full max-w-[1200px] flex-col gap-6 px-8 pt-7 pb-10">
      <header className="flex items-end justify-between gap-4">
        <div className="min-w-0">
          <h1 className="font-display text-2xl font-semibold tracking-tight text-fg">{title}</h1>
          <p className="mt-1 text-sm text-fg-muted">{description}</p>
        </div>
        {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
      </header>
      {children}
    </div>
  );
}

interface PanelProps {
  title?: string;
  description?: string;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
  bodyClassName?: string;
}

/** Cartão: superfície elevada com borda discreta e sombra mínima. */
export function Panel({
  title,
  description,
  actions,
  children,
  className,
  bodyClassName,
}: PanelProps) {
  const headingId = title ? `panel-${title.replace(/\W+/g, "-").toLowerCase()}` : undefined;
  return (
    <section
      aria-labelledby={headingId}
      className={cn(
        "rounded-xl border border-border bg-surface-1 [box-shadow:var(--shadow-card),var(--highlight-top)]",
        className,
      )}
    >
      {(title ?? actions) && (
        <div className="flex items-center justify-between gap-3 border-b border-border px-5 py-3.5">
          <div className="min-w-0">
            {title && (
              <h2 id={headingId} className="text-sm font-semibold tracking-tight text-fg">
                {title}
              </h2>
            )}
            {description && <p className="mt-0.5 text-xs text-fg-muted">{description}</p>}
          </div>
          {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
        </div>
      )}
      <div className={cn("px-5 py-4", bodyClassName)}>{children}</div>
    </section>
  );
}

export interface KeyValueItem {
  label: string;
  value: ReactNode;
  mono?: boolean;
}

export function KeyValueList({ items, className }: { items: KeyValueItem[]; className?: string }) {
  return (
    <dl className={cn("grid grid-cols-[minmax(140px,max-content)_1fr] gap-x-6 gap-y-2", className)}>
      {items.map((item) => (
        <div key={item.label} className="contents">
          <dt className="text-sm text-fg-muted">{item.label}</dt>
          <dd
            className={cn(
              "selectable min-w-0 text-sm break-all whitespace-pre-line text-fg",
              item.mono && "font-mono text-xs leading-5",
            )}
          >
            {item.value}
          </dd>
        </div>
      ))}
    </dl>
  );
}

interface StatCardProps {
  label: string;
  value: ReactNode;
  icon: LucideIcon;
  hint?: ReactNode;
  tone?: "accent" | "success" | "warning" | "danger" | "neutral";
  onClick?: () => void;
}

const STAT_TONES: Record<NonNullable<StatCardProps["tone"]>, string> = {
  accent: "bg-accent-subtle text-accent-text",
  success: "bg-success-subtle text-success",
  warning: "bg-warning-subtle text-warning",
  danger: "bg-danger-subtle text-danger",
  neutral: "bg-surface-3 text-fg-muted",
};

/** Indicador numérico com ícone; clicável quando leva à seção correspondente. */
export function StatCard({
  label,
  value,
  icon: Icon,
  hint,
  tone = "accent",
  onClick,
}: StatCardProps) {
  const body = (
    <>
      <div className="flex items-center justify-between gap-3">
        <span className="text-xs font-medium text-fg-muted">{label}</span>
        <span
          className={cn("flex size-7 items-center justify-center rounded-lg", STAT_TONES[tone])}
          aria-hidden="true"
        >
          <Icon className="size-4" />
        </span>
      </div>
      <div className="mt-2 font-display text-3xl font-semibold tracking-tight text-fg tabular-nums">
        {value}
      </div>
      {hint && <div className="mt-0.5 truncate text-xs text-fg-subtle">{hint}</div>}
    </>
  );
  const className =
    "flex min-w-0 flex-col rounded-xl border border-border bg-surface-1 px-4 py-3.5 text-left [box-shadow:var(--shadow-card),var(--highlight-top)]";
  if (onClick) {
    return (
      <button
        type="button"
        onClick={onClick}
        className={cn(
          className,
          "transition-[border-color,box-shadow,transform] duration-[var(--duration-base)] hover:-translate-y-0.5 hover:border-border-strong hover:shadow-raised",
        )}
      >
        {body}
      </button>
    );
  }
  return <div className={className}>{body}</div>;
}
