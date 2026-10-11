import { ArrowRight, CircleCheck, CircleDashed, Inbox } from "lucide-react";

import { EmptyState, ErrorState, StatusDot } from "@/components/feedback";
import { KeyValueList, Page, Panel } from "@/components/layout";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { OperationsTable } from "@/features/operations/OperationsTable";
import { formatBytes, formatDateTime } from "@/lib/format";
import { t } from "@/lib/i18n/pt-BR";
import { operationQuery, useDiagnostics, useOperations, useStartupStatus } from "@/lib/queries";
import { integrityPassed } from "@/features/operations/OperationsTable";
import { useUi } from "@/stores/ui";

const RECENT = operationQuery({ limit: 6 });

export function OverviewPage() {
  return (
    <Page title={t.nav.overview.label} description={t.nav.overview.description}>
      <div className="grid gap-5 xl:grid-cols-[minmax(0,1.15fr)_minmax(0,1fr)]">
        <div className="flex flex-col gap-5">
          <SystemPanel />
          <StepsPanel />
        </div>
        <RecentOperationsPanel />
      </div>
    </Page>
  );
}

function SystemPanel() {
  const startup = useStartupStatus();
  const diagnostics = useDiagnostics();

  if (diagnostics.isError) {
    return (
      <Panel title={t.overview.systemTitle}>
        <ErrorState error={diagnostics.error} onRetry={() => void diagnostics.refetch()} />
      </Panel>
    );
  }
  const d = diagnostics.data;
  const s = startup.data;
  const lastCheck = d?.lastIntegrityCheck;
  const checkOk = lastCheck ? integrityPassed(lastCheck) : false;

  return (
    <Panel title={t.overview.systemTitle}>
      {!d ? (
        <div className="flex flex-col gap-2.5" aria-busy="true" aria-label={t.common.loading}>
          <Skeleton className="h-4 w-2/3" />
          <Skeleton className="h-4 w-1/2" />
          <Skeleton className="h-4 w-3/5" />
        </div>
      ) : (
        <KeyValueList
          items={[
            {
              label: t.overview.database,
              value: (
                <span className="flex items-center gap-1.5">
                  <StatusDot tone="success" />
                  {t.overview.databaseReady}
                </span>
              ),
            },
            {
              label: t.overview.schema,
              value: t.diagnostics.schemaVersion(d.schemaVersion, d.latestSchemaVersion),
            },
            {
              label: t.overview.integrity,
              value: lastCheck
                ? `${checkOk ? t.overview.integrityOk : t.overview.integrityFail} · ${formatDateTime(lastCheck.startedAt)}`
                : t.overview.integrityNever,
            },
            {
              label: t.overview.storage,
              value: formatBytes(d.database.sizeBytes + d.wal.sizeBytes + d.logsSizeBytes),
            },
          ]}
        />
      )}
      {s && (s.appliedMigrations.length > 0 || s.backupCreated || s.recoveredOperations > 0) && (
        <ul className="mt-3 flex flex-col gap-1 border-t border-border pt-3 text-xs text-fg-muted">
          {s.appliedMigrations.length > 0 && (
            <li>
              {t.overview.startupMigrated(s.appliedMigrations.map((v) => `v${v}`).join(", "))}
            </li>
          )}
          {s.backupCreated && <li>{t.overview.startupBackup}</li>}
          {s.recoveredOperations > 0 && (
            <li>{t.overview.startupRecovered(s.recoveredOperations)}</li>
          )}
        </ul>
      )}
    </Panel>
  );
}

function StepsPanel() {
  return (
    <Panel title={t.overview.stepsTitle} description={t.overview.stepsDescription}>
      <ol className="flex flex-col">
        <li className="flex gap-3 py-2">
          <CircleCheck className="mt-0.5 size-4 shrink-0 text-success" aria-hidden="true" />
          <div className="min-w-0 flex-1">
            <p className="text-sm font-medium text-fg">{t.overview.foundationTitle}</p>
            <p className="text-sm text-fg-muted">{t.overview.foundationBody}</p>
          </div>
          <Badge tone="success" className="self-start">
            {t.overview.available}
          </Badge>
        </li>
        {t.overview.steps.map((step) => (
          <li key={step.title} className="flex gap-3 border-t border-border py-2">
            <CircleDashed className="mt-0.5 size-4 shrink-0 text-fg-subtle" aria-hidden="true" />
            <div className="min-w-0 flex-1">
              <p className="text-sm font-medium text-fg">{step.title}</p>
              <p className="text-sm text-fg-muted">{step.body}</p>
            </div>
            <Badge className="self-start">{t.nav.phaseBadge(step.phase)}</Badge>
          </li>
        ))}
      </ol>
    </Panel>
  );
}

function RecentOperationsPanel() {
  const operations = useOperations(RECENT);
  const navigate = useUi((s) => s.navigate);
  return (
    <Panel
      title={t.overview.recentTitle}
      actions={
        <Button variant="ghost" size="sm" onClick={() => navigate("diagnostics")}>
          {t.overview.viewAll}
          <ArrowRight aria-hidden="true" />
        </Button>
      }
    >
      {operations.isError ? (
        <ErrorState error={operations.error} onRetry={() => void operations.refetch()} />
      ) : !operations.data ? (
        <div className="flex flex-col gap-2" aria-busy="true">
          {Array.from({ length: 4 }, (_, i) => (
            <Skeleton key={i} className="h-5" />
          ))}
        </div>
      ) : operations.data.length === 0 ? (
        <EmptyState icon={Inbox} title={t.overview.recentEmpty} />
      ) : (
        <OperationsTable operations={operations.data} compact />
      )}
    </Panel>
  );
}
