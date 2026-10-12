import { Lock } from "lucide-react";

import { StatusDot } from "@/components/feedback";
import { Tooltip } from "@/components/ui/tooltip";
import { t } from "@/lib/i18n/pt-BR";
import { formatInteger } from "@/lib/format";
import {
  operationQuery,
  useAppInfo,
  useOperations,
  useProjects,
  useStartupStatus,
} from "@/lib/queries";
import { useIndexing } from "@/stores/indexing";

const RUNNING_QUERY = operationQuery({ status: "running", limit: 50 });
const POLL_MS = 5_000;

export function StatusBar() {
  const startup = useStartupStatus();
  const appInfo = useAppInfo();
  const running = useOperations(RUNNING_QUERY, POLL_MS);

  let db: { tone: "success" | "danger" | "neutral"; label: string };
  if (startup.data?.ready && startup.data.schemaVersion !== null) {
    db = { tone: "success", label: t.status.dbOk(startup.data.schemaVersion) };
  } else if (startup.data && !startup.data.ready) {
    db = { tone: "danger", label: t.status.dbUnavailable };
  } else {
    db = { tone: "neutral", label: t.status.dbLoading };
  }
  const runningCount = running.data?.length ?? 0;
  const queue = useIndexing((s) => s.queue);
  const liveJob = useIndexing((s) =>
    s.queue.running === null ? undefined : s.live[s.queue.running],
  );
  const projects = useProjects();
  const runningName =
    queue.running === null
      ? null
      : (projects.data?.find((p) => p.project.id === queue.running)?.project.name ?? "");
  let indexing: string | null = null;
  if (runningName !== null) {
    indexing = queue.paused
      ? t.status.indexingPaused(runningName)
      : t.status.indexing(
          runningName,
          liveJob && liveJob.phase === "indexing"
            ? `${formatInteger(liveJob.processed)}/${formatInteger(liveJob.toProcess)}`
            : "",
        );
  }

  return (
    <footer className="flex h-7 shrink-0 items-center gap-4 border-t border-border bg-surface-sidebar px-4 text-xs text-fg-muted">
      <span className="flex items-center gap-1.5">
        <StatusDot tone={db.tone} />
        {db.label}
      </span>
      {indexing !== null ? (
        <span className="flex items-center gap-1.5" data-testid="status-indexing">
          <StatusDot tone="warning" />
          {indexing}
          {queue.queued.length > 0 && ` · ${t.status.queued(queue.queued.length)}`}
        </span>
      ) : (
        <span className="flex items-center gap-1.5">
          <StatusDot tone={runningCount > 0 ? "warning" : "neutral"} />
          {runningCount > 0 ? t.status.running(runningCount) : t.status.noRunning}
        </span>
      )}
      <span className="flex-1" />
      <Tooltip content={t.status.localOnlyTooltip}>
        <span className="flex items-center gap-1" tabIndex={0}>
          <Lock className="size-3" aria-hidden="true" />
          {t.status.localOnly}
        </span>
      </Tooltip>
      {appInfo.data && (
        <span className="font-mono">
          {t.status.version(appInfo.data.appVersion, appInfo.data.buildProfile)}
        </span>
      )}
    </footer>
  );
}
