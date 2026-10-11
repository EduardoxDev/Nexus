import type { OperationRecord } from "@/lib/bindings";
import { OperationStatusBadge } from "@/components/feedback";
import { cn } from "@/lib/cn";
import { formatDateTime, formatDuration, formatRelative } from "@/lib/format";
import { t } from "@/lib/i18n/pt-BR";

/** Uma verificação de integridade passou se concluiu e registrou `ok: true`. */
export function integrityPassed(op: OperationRecord): boolean {
  const detail = op.detail;
  return (
    op.status === "succeeded" &&
    typeof detail === "object" &&
    detail !== null &&
    !Array.isArray(detail) &&
    detail.ok === true
  );
}

export function operationKindLabel(kind: string): string {
  return t.operationKinds[kind] ?? kind;
}

interface OperationsTableProps {
  operations: OperationRecord[];
  selectedId?: string | null;
  onSelect?: (op: OperationRecord) => void;
  compact?: boolean;
}

export function OperationsTable({
  operations,
  selectedId,
  onSelect,
  compact,
}: OperationsTableProps) {
  const [whenCol, kindCol, statusCol, durationCol] = t.diagnostics.operationColumns;
  return (
    <table className="w-full table-fixed border-collapse text-sm">
      <thead>
        <tr className="text-left text-xs text-fg-subtle">
          <th scope="col" className="w-[30%] pb-2 font-medium">
            {whenCol}
          </th>
          <th scope="col" className="pb-2 font-medium">
            {kindCol}
          </th>
          <th scope="col" className="w-28 pb-2 font-medium">
            {statusCol}
          </th>
          {!compact && (
            <th scope="col" className="w-24 pb-2 text-right font-medium">
              {durationCol}
            </th>
          )}
        </tr>
      </thead>
      <tbody>
        {operations.map((op) => {
          const selected = op.id === selectedId;
          return (
            <tr
              key={op.id}
              onClick={onSelect ? () => onSelect(op) : undefined}
              onKeyDown={
                onSelect
                  ? (e) => {
                      if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        onSelect(op);
                      }
                    }
                  : undefined
              }
              tabIndex={onSelect ? 0 : undefined}
              aria-selected={onSelect ? selected : undefined}
              className={cn(
                "border-t border-border",
                onSelect && "cursor-default hover:bg-surface-2",
                selected && "bg-accent-subtle hover:bg-accent-subtle",
              )}
            >
              <td className="py-1.5 pr-2 text-fg-muted" title={formatDateTime(op.startedAt)}>
                {formatRelative(op.startedAt)}
              </td>
              <td className="truncate py-1.5 pr-2 text-fg">{operationKindLabel(op.kind)}</td>
              <td className="py-1.5 pr-2">
                <OperationStatusBadge status={op.status} />
              </td>
              {!compact && (
                <td className="py-1.5 text-right font-mono text-xs text-fg-muted">
                  {op.durationMs === null ? "—" : formatDuration(op.durationMs)}
                </td>
              )}
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}
