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
    <div className="mx-auto flex w-full max-w-[1120px] flex-col gap-5 px-6 py-5">
      <header className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <h1 className="text-xl font-semibold tracking-tight text-fg">{title}</h1>
          <p className="mt-0.5 text-sm text-fg-muted">{description}</p>
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

/** Seção com borda discreta. Sem sombra: superfícies e bordas fazem a hierarquia. */
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
      className={cn("rounded-lg border border-border bg-surface-1", className)}
    >
      {(title ?? actions) && (
        <div className="flex items-start justify-between gap-3 border-b border-border px-4 py-3">
          <div className="min-w-0">
            {title && (
              <h2 id={headingId} className="text-base font-semibold text-fg">
                {title}
              </h2>
            )}
            {description && <p className="mt-0.5 text-xs text-fg-muted">{description}</p>}
          </div>
          {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
        </div>
      )}
      <div className={cn("px-4 py-3", bodyClassName)}>{children}</div>
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
