import {
  AlertTriangle,
  ArrowRight,
  Brain,
  ChevronRight,
  FileCode2,
  FolderGit2,
  FolderPlus,
  Inbox,
  Lock,
  Package,
  Search,
} from "lucide-react";

import { routeDef } from "@/app/routes";
import logoUrl from "@/assets/nexus-icon-128.png";
import { EmptyState, ErrorState, StatusDot } from "@/components/feedback";
import { KeyValueList, Page, Panel, StatCard } from "@/components/layout";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { OperationsTable, integrityPassed } from "@/features/operations/OperationsTable";
import { activityLine } from "@/features/projects/activity";
import type { ProjectSummary } from "@/lib/bindings";
import { formatBytes, formatDateTime, formatInteger } from "@/lib/format";
import { t } from "@/lib/i18n/pt-BR";
import {
  operationQuery,
  useDiagnostics,
  useOperations,
  useProjects,
  useStartupStatus,
  useWorkspaceTotals,
} from "@/lib/queries";
import { useIndexing, useProjectActivity } from "@/stores/indexing";
import { useUi } from "@/stores/ui";

const RECENT = operationQuery({ limit: 6 });

export function OverviewPage() {
  return (
    <Page title={t.nav.overview.label} description={t.nav.overview.description}>
      <Hero />
      <KpiRow />
      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.25fr)_minmax(0,1fr)]">
        <div className="flex min-w-0 flex-col gap-6">
          <ProjectsPanel />
          <RecentOperationsPanel />
        </div>
        <div className="flex min-w-0 flex-col gap-6">
          <SystemPanel />
          <StepsPanel />
        </div>
      </div>
    </Page>
  );
}

function Hero() {
  const navigate = useUi((s) => s.navigate);
  return (
    <section
      aria-label={t.app.name}
      className="relative overflow-hidden rounded-2xl border border-border bg-surface-1 px-7 py-6 [box-shadow:var(--shadow-card),var(--highlight-top)]"
    >
      {/* Brilho da marca: decorativo. */}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 bg-[radial-gradient(60%_120%_at_100%_0%,var(--accent-glow),transparent_60%),radial-gradient(40%_90%_at_0%_100%,var(--accent-glow),transparent_70%)]"
      />
      <div className="relative flex flex-wrap items-center gap-x-8 gap-y-5">
        <div className="flex min-w-0 flex-1 items-center gap-5">
          <img
            src={logoUrl}
            alt=""
            aria-hidden="true"
            className="size-16 shrink-0 rounded-2xl shadow-raised"
            draggable={false}
          />
          <div className="min-w-0">
            <p className="font-display text-3xl font-bold tracking-tight text-fg">{t.app.name}</p>
            <p className="text-base font-medium text-accent-text">{t.app.slogan}</p>
            <p className="mt-0.5 flex flex-wrap items-center gap-x-1.5 text-sm text-fg-muted">
              <span>{t.app.tagline}</span>
              <span aria-hidden="true">·</span>
              <span className="flex items-center gap-1">
                <Lock className="size-3.5" aria-hidden="true" />
                {t.overview.heroLocal}
              </span>
            </p>
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button variant="primary" size="lg" onClick={() => navigate("search")}>
            <Search aria-hidden="true" />
            {t.overview.heroSearch}
          </Button>
          <Button size="lg" onClick={() => navigate("contextPacks")}>
            <Package aria-hidden="true" />
            {t.overview.heroPack}
          </Button>
        </div>
      </div>
    </section>
  );
}

/** Indicadores somados sobre todos os projetos; cada um leva à seção correspondente. */
function KpiRow() {
  const projects = useProjects();
  const navigate = useUi((s) => s.navigate);
  const indexingNow = useIndexing(
    (s) => (s.queue.running === null ? 0 : 1) + s.queue.queued.length,
  );
  const list = projects.data;
  const totals = useWorkspaceTotals(list?.map((p) => p.project.id) ?? []);
  const files = list?.reduce((n, p) => n + p.counts.indexed, 0) ?? 0;
  const bytes = list?.reduce((n, p) => n + p.counts.indexedBytes, 0) ?? 0;
  const lines = list?.reduce((n, p) => n + p.counts.indexedLines, 0) ?? 0;
  const loading = <Skeleton className="mt-1 h-8 w-16" />;

  return (
    <section aria-label={t.overview.kpiTitle} className="grid grid-cols-2 gap-4 xl:grid-cols-4">
      <StatCard
        label={t.overview.kpiProjects}
        icon={FolderGit2}
        value={list ? formatInteger(list.length) : loading}
        hint={t.overview.kpiProjectsHint(indexingNow)}
        onClick={() => navigate("projects")}
      />
      <StatCard
        label={t.overview.kpiFiles}
        icon={FileCode2}
        tone="neutral"
        value={list ? formatInteger(files) : loading}
        hint={list ? t.overview.kpiFilesHint(formatBytes(bytes), formatInteger(lines)) : undefined}
        onClick={() => navigate("search")}
      />
      <StatCard
        label={t.overview.kpiMemory}
        icon={Brain}
        tone="success"
        value={list && totals ? formatInteger(totals.confirmed) : loading}
        hint={totals ? t.overview.kpiMemoryHint(totals.proposed) : undefined}
        onClick={() => navigate("memory")}
      />
      <StatCard
        label={t.overview.kpiAlerts}
        icon={AlertTriangle}
        tone={totals && totals.openIssues > 0 ? "warning" : "success"}
        value={list && totals ? formatInteger(totals.openIssues) : loading}
        hint={totals ? t.overview.kpiAlertsHint(totals.contradictions) : undefined}
        onClick={() => navigate("freshness")}
      />
    </section>
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
                <Badge tone="success">
                  <StatusDot tone="success" />
                  {t.overview.databaseReady}
                </Badge>
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
  const navigate = useUi((s) => s.navigate);
  return (
    <Panel
      title={t.overview.stepsTitle}
      description={t.overview.stepsDescription}
      bodyClassName="p-2"
    >
      <ol className="flex flex-col">
        {t.overview.steps.map((step, index) => {
          const Icon = routeDef(step.route).icon;
          return (
            <li key={step.title}>
              <button
                type="button"
                onClick={() => navigate(step.route)}
                aria-label={`${t.overview.open}: ${step.title}`}
                className="group flex w-full items-start gap-3 rounded-lg px-3 py-2.5 text-left transition-colors hover:bg-surface-2"
              >
                <span className="relative mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg bg-accent-subtle text-accent-text">
                  <Icon className="size-4" aria-hidden="true" />
                  <span className="absolute -top-1 -right-1 flex size-4 items-center justify-center rounded-full bg-accent text-[10px] font-bold text-accent-fg">
                    {index + 1}
                  </span>
                </span>
                <span className="min-w-0 flex-1">
                  <span className="block text-sm font-medium text-fg">{step.title}</span>
                  <span className="block text-xs leading-[18px] text-fg-muted">{step.body}</span>
                </span>
                <ChevronRight
                  className="mt-2 size-4 shrink-0 text-fg-subtle transition-transform group-hover:translate-x-0.5 group-hover:text-fg"
                  aria-hidden="true"
                />
              </button>
            </li>
          );
        })}
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

function ProjectsPanel() {
  const projects = useProjects();
  const navigate = useUi((s) => s.navigate);
  const selectProject = useUi((s) => s.selectProject);
  const open = (id: number | null) => {
    if (id !== null) selectProject(id);
    navigate("projects");
  };
  return (
    <Panel
      title={t.projects.overviewTitle}
      actions={
        <>
          <Button size="sm" onClick={() => open(null)}>
            <FolderPlus aria-hidden="true" />
            {t.overview.heroAddProject}
          </Button>
          <Button variant="ghost" size="sm" onClick={() => open(null)}>
            {t.projects.overviewAll}
            <ArrowRight aria-hidden="true" />
          </Button>
        </>
      }
      bodyClassName="px-3 py-2"
    >
      {projects.isError ? (
        <ErrorState error={projects.error} onRetry={() => void projects.refetch()} />
      ) : !projects.data ? (
        <Skeleton className="h-10" />
      ) : projects.data.length === 0 ? (
        <EmptyState
          icon={FolderGit2}
          title={t.projects.overviewEmpty}
          action={
            <Button variant="primary" onClick={() => open(null)}>
              {t.projects.add}
            </Button>
          }
        />
      ) : (
        <ul className="flex flex-col">
          {projects.data.slice(0, 6).map((p) => (
            <li key={p.project.id}>
              <ProjectLine summary={p} onOpen={() => open(p.project.id)} />
            </li>
          ))}
        </ul>
      )}
    </Panel>
  );
}

function ProjectLine({ summary, onOpen }: { summary: ProjectSummary; onOpen: () => void }) {
  const activity = useProjectActivity(summary.project.id);
  return (
    <button
      type="button"
      onClick={onOpen}
      className="group flex w-full items-center gap-3 rounded-lg px-2 py-2 text-left transition-colors hover:bg-surface-2"
    >
      <span className="flex size-9 shrink-0 items-center justify-center rounded-lg border border-border bg-surface-2 text-fg-muted">
        <FolderGit2 className="size-4" aria-hidden="true" />
      </span>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-sm font-medium text-fg">{summary.project.name}</span>
        <span className="block truncate text-xs text-fg-muted">
          {activityLine(summary, activity)}
        </span>
      </span>
      {!summary.rootExists ? (
        <Badge tone="danger">{t.overview.projectMissing}</Badge>
      ) : summary.activeJob ? (
        <Badge tone="warning">
          <StatusDot tone="warning" />
          {t.overview.projectIndexing}
        </Badge>
      ) : (
        <Badge tone="success">
          <StatusDot tone="success" />
          {t.overview.projectReady}
        </Badge>
      )}
      <ChevronRight
        className="size-4 shrink-0 text-fg-subtle transition-transform group-hover:translate-x-0.5"
        aria-hidden="true"
      />
    </button>
  );
}
