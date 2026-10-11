import { Lock } from "lucide-react";

import { StatusDot } from "@/components/feedback";
import { Tooltip } from "@/components/ui/tooltip";
import { t } from "@/lib/i18n/pt-BR";
import { operationQuery, useAppInfo, useOperations, useStartupStatus } from "@/lib/queries";

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

  return (
    <footer
      role="status"
      aria-live="polite"
      className="flex h-7 shrink-0 items-center gap-4 border-t border-border bg-surface-1 px-3 text-xs text-fg-muted"
    >
      <span className="flex items-center gap-1.5">
        <StatusDot tone={db.tone} />
        {db.label}
      </span>
      <span className="flex items-center gap-1.5">
        <StatusDot tone={runningCount > 0 ? "warning" : "neutral"} />
        {runningCount > 0 ? t.status.running(runningCount) : t.status.noRunning}
      </span>
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
