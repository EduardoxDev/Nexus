import { CircleAlert, Copy, Check, type LucideIcon } from "lucide-react";
import { useState, type ReactNode } from "react";

import { Button } from "@/components/ui/button";
import type { OperationStatus } from "@/lib/bindings";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/cn";
import { t } from "@/lib/i18n/pt-BR";
import { toNexusError } from "@/lib/ipc";

interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  description?: string;
  action?: ReactNode;
}

export function EmptyState({ icon: Icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center gap-2 px-6 py-10 text-center">
      <Icon className="size-5 text-fg-subtle" aria-hidden="true" />
      <p className="text-sm font-medium text-fg">{title}</p>
      {description && <p className="max-w-md text-sm text-fg-muted">{description}</p>}
      {action && <div className="mt-2">{action}</div>}
    </div>
  );
}

export function CopyButton({ value, label = t.common.copy }: { value: string; label?: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <Button
      variant="ghost"
      size="sm"
      onClick={() => {
        void navigator.clipboard
          .writeText(value)
          .then(() => {
            setCopied(true);
            window.setTimeout(() => setCopied(false), 1500);
          })
          .catch(() => setCopied(false));
      }}
    >
      {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
      {copied ? t.common.copied : label}
    </Button>
  );
}

interface ErrorStateProps {
  error: unknown;
  onRetry?: () => void;
  className?: string;
}

/** Erro com código, `opId` (para achar a operação no Diagnóstico) e cópia dos detalhes. */
export function ErrorState({ error, onRetry, className }: ErrorStateProps) {
  const err = toNexusError(error);
  const details = JSON.stringify({ code: err.code, message: err.message, opId: err.opId }, null, 2);
  return (
    <div
      role="alert"
      className={cn(
        "flex gap-3 rounded-md border border-danger/40 bg-danger-subtle px-3 py-2.5",
        className,
      )}
    >
      <CircleAlert className="mt-0.5 size-4 shrink-0 text-danger" aria-hidden="true" />
      <div className="min-w-0 flex-1">
        <p className="text-sm font-medium text-fg">{t.common.errorTitle}</p>
        <p className="selectable text-sm text-fg-muted">{err.message}</p>
        <p className="selectable mt-1 font-mono text-xs text-fg-subtle">
          {err.code}
          {err.opId && ` · ${t.common.operation} ${err.opId}`}
        </p>
        <div className="mt-2 flex gap-2">
          {onRetry && (
            <Button size="sm" onClick={onRetry}>
              {t.common.retry}
            </Button>
          )}
          <CopyButton value={details} label={t.common.copyDetails} />
        </div>
      </div>
    </div>
  );
}

const statusTone: Record<OperationStatus, "neutral" | "success" | "danger" | "warning" | "info"> = {
  running: "info",
  succeeded: "success",
  failed: "danger",
  cancelled: "neutral",
  interrupted: "warning",
};

export function OperationStatusBadge({ status }: { status: OperationStatus }) {
  return <Badge tone={statusTone[status]}>{t.operationStatus[status]}</Badge>;
}

export function StatusDot({ tone }: { tone: "success" | "danger" | "warning" | "neutral" }) {
  return (
    <span
      aria-hidden="true"
      className={cn(
        "inline-block size-2 shrink-0 rounded-full",
        tone === "success" && "bg-success",
        tone === "danger" && "bg-danger",
        tone === "warning" && "bg-warning",
        tone === "neutral" && "bg-fg-subtle",
      )}
    />
  );
}
