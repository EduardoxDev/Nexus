import { cn } from "@/lib/cn";

interface ProgressProps {
  /** 0..1; `null` = indeterminado (fase sem total conhecido). */
  value: number | null;
  label: string;
  className?: string;
}

/** Barra de progresso fina. O valor exibido é sempre o real (sem animação de "falso progresso"). */
export function Progress({ value, label, className }: ProgressProps) {
  const pct = value === null ? null : Math.round(Math.min(1, Math.max(0, value)) * 100);
  return (
    <div
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      {...(pct === null ? {} : { "aria-valuenow": pct })}
      className={cn("relative h-1.5 w-full overflow-hidden rounded-full bg-surface-3", className)}
    >
      {pct === null ? (
        <div className="absolute inset-y-0 w-1/3 animate-[nexus-indeterminate_1.4s_ease-in-out_infinite] rounded-full bg-accent/70" />
      ) : (
        <div
          className="h-full rounded-full bg-accent transition-[width] duration-[var(--duration-base)]"
          style={{ width: `${pct}%` }}
        />
      )}
    </div>
  );
}
