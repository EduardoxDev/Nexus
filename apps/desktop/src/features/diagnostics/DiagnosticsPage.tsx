import { AnimatePresence, motion } from "motion/react";
import { Inbox, RefreshCw, ShieldCheck, X } from "lucide-react";
import { useState } from "react";

import {
  CopyButton,
  EmptyState,
  ErrorState,
  OperationStatusBadge,
  StatusDot,
} from "@/components/feedback";
import { KeyValueList, Page, Panel } from "@/components/layout";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Segmented, type SegmentedOption } from "@/components/ui/segmented";
import { Skeleton } from "@/components/ui/skeleton";
import { OperationsTable, operationKindLabel } from "@/features/operations/OperationsTable";
import type { DiagnosticsReport, OperationRecord, OperationStatus } from "@/lib/bindings";
import {
  formatBytes,
  formatDateTime,
  formatDuration,
  formatRelative,
  shortHash,
} from "@/lib/format";
import { t } from "@/lib/i18n/pt-BR";
import {
  operationQuery,
  useAppInfo,
  useDiagnostics,
  useIntegrityCheck,
  useOperations,
} from "@/lib/queries";

type StatusFilter = "all" | OperationStatus;

const FILTERS: readonly SegmentedOption<StatusFilter>[] = [
  { value: "all", label: t.diagnostics.filterAll },
  { value: "succeeded", label: t.operationStatus.succeeded },
  { value: "failed", label: t.operationStatus.failed },
  { value: "interrupted", label: t.operationStatus.interrupted },
  { value: "running", label: t.operationStatus.running },
];

export function DiagnosticsPage() {
  const diagnostics = useDiagnostics();
  const [filter, setFilter] = useState<StatusFilter>("all");
  const operations = useOperations(
    operationQuery({ limit: 100, status: filter === "all" ? null : filter }),
  );
  const [selected, setSelected] = useState<OperationRecord | null>(null);

  const refresh = () => {
    void diagnostics.refetch();
    void operations.refetch();
  };

  return (
    <div className="flex h-full min-h-0">
      <div className="min-w-0 flex-1 overflow-y-auto">
        <Page
          title={t.nav.diagnostics.label}
          description={t.nav.diagnostics.description}
          actions={
            <Button onClick={refresh} disabled={diagnostics.isFetching}>
              <RefreshCw
                aria-hidden="true"
                className={diagnostics.isFetching ? "animate-spin" : ""}
              />
              {t.diagnostics.refresh}
            </Button>
          }
        >
          {diagnostics.isError ? (
            <ErrorState error={diagnostics.error} onRetry={() => void diagnostics.refetch()} />
          ) : !diagnostics.data ? (
            <DiagnosticsSkeleton />
          ) : (
            <>
              <div className="grid gap-5 xl:grid-cols-2">
                <AppPanel />
                <StoragePanel report={diagnostics.data} />
              </div>
              <IntegrityPanel report={diagnostics.data} />
              <MigrationsPanel report={diagnostics.data} />
            </>
          )}

          <Panel
            title={t.diagnostics.operations}
            description={t.diagnostics.operationsDescription}
            actions={
              <Segmented
                aria-label={t.diagnostics.operations}
                value={filter}
                options={FILTERS}
                onValueChange={(v) => {
                  setFilter(v);
                  setSelected(null);
                }}
              />
            }
          >
            {operations.isError ? (
              <ErrorState error={operations.error} onRetry={() => void operations.refetch()} />
            ) : !operations.data ? (
              <div className="flex flex-col gap-2" aria-busy="true">
                {Array.from({ length: 5 }, (_, i) => (
                  <Skeleton key={i} className="h-5" />
                ))}
              </div>
            ) : operations.data.length === 0 ? (
              <EmptyState icon={Inbox} title={t.diagnostics.operationsEmpty} />
            ) : (
              <OperationsTable
                operations={operations.data}
                selectedId={selected?.id ?? null}
                onSelect={(op) => setSelected((cur) => (cur?.id === op.id ? null : op))}
              />
            )}
          </Panel>
        </Page>
      </div>

      <AnimatePresence initial={false}>
        {selected && (
          <motion.aside
            key="detail"
            aria-label={t.diagnostics.detailTitle}
            initial={{ width: 0, opacity: 0 }}
            animate={{ width: 360, opacity: 1 }}
            exit={{ width: 0, opacity: 0 }}
            transition={{ duration: 0.16, ease: "easeOut" }}
            className="shrink-0 overflow-hidden border-l border-border bg-surface-1"
          >
            <OperationDetail op={selected} onClose={() => setSelected(null)} />
          </motion.aside>
        )}
      </AnimatePresence>
    </div>
  );
}

function AppPanel() {
  const info = useAppInfo();
  const d = info.data;
  return (
    <Panel title={t.diagnostics.app}>
      {!d ? (
        <Skeleton className="h-24" />
      ) : (
        <KeyValueList
          items={[
            { label: t.diagnostics.appVersion, value: d.appVersion, mono: true },
            { label: t.diagnostics.coreVersion, value: d.coreVersion, mono: true },
            { label: t.diagnostics.profile, value: d.buildProfile, mono: true },
            { label: t.diagnostics.os, value: `${d.os} ${d.arch}`, mono: true },
            {
              label: t.diagnostics.webview,
              value: d.webviewVersion ?? t.common.unknown,
              mono: true,
            },
            { label: t.diagnostics.tauri, value: d.tauriVersion ?? t.common.unknown, mono: true },
          ]}
        />
      )}
    </Panel>
  );
}

function StoragePanel({ report: d }: { report: DiagnosticsReport }) {
  return (
    <Panel title={t.diagnostics.storage}>
      <KeyValueList
        items={[
          { label: t.diagnostics.dataDir, value: d.dataDir, mono: true },
          {
            label: t.diagnostics.database,
            value: `${formatBytes(d.database.sizeBytes)} · ${t.diagnostics.wal} ${formatBytes(d.wal.sizeBytes)}`,
          },
          {
            label: t.diagnostics.logs,
            value: `${t.diagnostics.logFiles(d.logFiles)} · ${formatBytes(d.logsSizeBytes)}`,
          },
          {
            label: t.diagnostics.backups,
            value:
              d.backups.length === 0
                ? t.diagnostics.backupsEmpty
                : d.backups.map((b) => `${b.fileName} (${formatBytes(b.sizeBytes)})`).join("\n"),
          },
          {
            label: t.diagnostics.sqlite,
            value: (
              <span className="flex items-center gap-2">
                <span className="font-mono text-xs">{d.sqliteVersion}</span>
                <Badge tone={d.fts5Available ? "success" : "danger"}>
                  {t.diagnostics.fts5}{" "}
                  {d.fts5Available ? t.diagnostics.available : t.diagnostics.unavailable}
                </Badge>
              </span>
            ),
          },
        ]}
      />
    </Panel>
  );
}

function IntegrityPanel({ report: d }: { report: DiagnosticsReport }) {
  const check = useIntegrityCheck();
  const result = check.data;
  return (
    <Panel
      title={t.diagnostics.integrity}
      description={t.diagnostics.integrityDescription}
      actions={
        <Button variant="primary" onClick={() => check.mutate()} disabled={check.isPending}>
          <ShieldCheck aria-hidden="true" />
          {check.isPending ? t.diagnostics.integrityRunning : t.diagnostics.integrityRun}
        </Button>
      }
    >
      <div aria-live="polite" className="flex flex-col gap-2 text-sm">
        {check.isError && <ErrorState error={check.error} />}
        {result && (
          <div className="flex items-start gap-2">
            <StatusDot tone={result.ok ? "success" : "danger"} />
            <div className="-mt-1.5">
              <p className="text-fg">
                {result.ok
                  ? t.diagnostics.integrityOk(formatDuration(result.durationMs))
                  : t.diagnostics.integrityFail(result.messages.length)}
              </p>
              {!result.ok && (
                <ul className="selectable mt-1 list-disc pl-4 font-mono text-xs text-fg-muted">
                  {result.messages.map((m) => (
                    <li key={m}>{m}</li>
                  ))}
                </ul>
              )}
            </div>
          </div>
        )}
        {!result && !check.isError && (
          <p className="text-fg-muted">
            {d.lastIntegrityCheck
              ? t.diagnostics.integrityLast(formatDateTime(d.lastIntegrityCheck.startedAt))
              : t.overview.integrityNever}
          </p>
        )}
      </div>
    </Panel>
  );
}

function MigrationsPanel({ report: d }: { report: DiagnosticsReport }) {
  const columns = t.diagnostics.migrationColumns;
  return (
    <Panel
      title={t.diagnostics.schema}
      actions={
        <Badge tone="accent">
          {t.diagnostics.schemaVersion(d.schemaVersion, d.latestSchemaVersion)}
        </Badge>
      }
    >
      <table className="w-full border-collapse text-sm">
        <thead>
          <tr className="text-left text-xs text-fg-subtle">
            {columns.map((c) => (
              <th key={c} scope="col" className="pb-2 font-medium">
                {c}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {d.migrations.map((m) => (
            <tr key={m.version} className="border-t border-border">
              <td className="py-1.5 font-mono text-xs">v{m.version}</td>
              <td className="py-1.5">{m.name}</td>
              <td className="py-1.5 text-fg-muted">{formatDateTime(m.appliedAt)}</td>
              <td className="py-1.5 font-mono text-xs text-fg-muted">
                {formatDuration(m.durationMs)}
              </td>
              <td className="selectable py-1.5 font-mono text-xs text-fg-muted" title={m.checksum}>
                {shortHash(m.checksum)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </Panel>
  );
}

function OperationDetail({ op, onClose }: { op: OperationRecord; onClose: () => void }) {
  return (
    <div className="flex h-full w-[360px] flex-col">
      <div className="flex h-12 items-center justify-between border-b border-border px-4">
        <h2 className="text-base font-semibold">{t.diagnostics.detailTitle}</h2>
        <Button variant="ghost" size="icon-sm" onClick={onClose} aria-label={t.common.close}>
          <X />
        </Button>
      </div>
      <div className="flex-1 overflow-y-auto px-4 py-3">
        <KeyValueList
          className="grid-cols-[96px_1fr]"
          items={[
            { label: t.diagnostics.detailKind, value: operationKindLabel(op.kind) },
            {
              label: t.diagnostics.detailStatus,
              value: <OperationStatusBadge status={op.status} />,
            },
            {
              label: t.diagnostics.detailStarted,
              value: `${formatDateTime(op.startedAt)} (${formatRelative(op.startedAt)})`,
            },
            {
              label: t.diagnostics.detailFinished,
              value: op.finishedAt === null ? "—" : formatDateTime(op.finishedAt),
            },
            {
              label: t.diagnostics.detailDuration,
              value: op.durationMs === null ? "—" : formatDuration(op.durationMs),
            },
            { label: t.diagnostics.detailId, value: op.id, mono: true },
            ...(op.errorCode || op.errorMessage
              ? [
                  {
                    label: t.diagnostics.detailError,
                    value: `${op.errorCode ?? ""} ${op.errorMessage ?? ""}`.trim(),
                  },
                ]
              : []),
          ]}
        />
        {op.detail !== null && (
          <div className="mt-4">
            <p className="mb-1.5 text-xs font-medium text-fg-subtle">{t.diagnostics.detailData}</p>
            <pre className="overflow-x-auto rounded-md border border-border bg-surface-2 p-2.5 font-mono text-xs leading-5 text-fg">
              {JSON.stringify(op.detail, null, 2)}
            </pre>
          </div>
        )}
        <div className="mt-3">
          <CopyButton value={JSON.stringify(op, null, 2)} label={t.common.copyDetails} />
        </div>
      </div>
    </div>
  );
}

function DiagnosticsSkeleton() {
  return (
    <div className="grid gap-5 xl:grid-cols-2" aria-busy="true" aria-label={t.common.loading}>
      <Skeleton className="h-48" />
      <Skeleton className="h-48" />
    </div>
  );
}
