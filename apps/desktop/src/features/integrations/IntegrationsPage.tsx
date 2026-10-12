import { Plug } from "lucide-react";

import { CopyButton, EmptyState, ErrorState } from "@/components/feedback";
import { Page, Panel } from "@/components/layout";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import type { IntegrationInfo, OperationRecord } from "@/lib/bindings";
import { formatDateTime, formatDuration, formatRelative } from "@/lib/format";
import { t } from "@/lib/i18n/pt-BR";
import { operationQuery, useIntegrationInfo, useOperations } from "@/lib/queries";

const s = t.integrations;

export function IntegrationsPage() {
  const info = useIntegrationInfo();
  return (
    <Page title={t.nav.integrations.label} description={s.description}>
      {info.isError && <ErrorState error={info.error} onRetry={() => void info.refetch()} />}
      {info.isPending && <Skeleton className="h-40" />}
      {info.data && <IntegrationsContent info={info.data} />}
    </Page>
  );
}

function CommandBlock({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-start gap-2 rounded-md border border-border bg-surface-2 p-2">
      <code aria-label={label} className="selectable min-w-0 flex-1 font-mono text-xs break-all">
        {value}
      </code>
      <CopyButton value={value} />
    </div>
  );
}

function IntegrationsContent({ info }: { info: IntegrationInfo }) {
  const calls = useOperations(operationQuery({ kindPrefix: "mcp.", limit: 20 }), 5_000);
  const config = JSON.stringify(
    {
      mcpServers: {
        nexus: { command: info.cliPath ?? "nexus", args: info.serveArgs },
      },
    },
    null,
    2,
  );
  return (
    <>
      <Panel title={s.claudeTitle} description={s.claudeDescription}>
        {info.claudeCommand ? (
          <div className="flex flex-col gap-2">
            <CommandBlock label={s.claudeCommandLabel} value={info.claudeCommand} />
            <p className="text-xs text-fg-muted">{s.claudeHint}</p>
          </div>
        ) : (
          <div role="alert" className="flex flex-col gap-2 text-sm">
            <p>{s.cliMissing}</p>
            <CommandBlock label={s.buildCli} value="cargo build -p nexus-cli --release" />
            <p className="text-xs text-fg-muted">{s.cliMissingHint}</p>
          </div>
        )}
      </Panel>

      <Panel title={s.otherTitle} description={s.otherDescription}>
        <pre className="selectable overflow-x-auto rounded-md border border-border bg-surface-2 p-2 font-mono text-xs">
          {config}
        </pre>
        <div className="mt-2 flex items-center justify-between gap-2">
          <p className="text-xs text-fg-muted">
            {s.protocol(info.protocolVersions[0] ?? "", info.protocolVersions.slice(1).join(", "))}
          </p>
          <CopyButton value={config} />
        </div>
      </Panel>

      <Panel title={s.toolsTitle} description={s.toolsDescription(info.tools.length)}>
        <ul className="flex flex-col divide-y divide-border" aria-label={s.toolsTitle}>
          {info.tools.map((tool) => (
            <li key={tool.name} className="flex flex-col gap-1 py-2">
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-mono text-sm">{tool.name}</span>
                <span className="text-sm text-fg-muted">{tool.title}</span>
                <Badge tone={tool.readOnly ? "success" : "warning"}>
                  {tool.readOnly ? s.readOnly : s.writesLocal}
                </Badge>
              </div>
              <p className="text-xs text-fg-muted">{tool.description}</p>
            </li>
          ))}
        </ul>
      </Panel>

      <Panel title={s.securityTitle}>
        <ul className="list-disc pl-5 text-sm text-fg-muted">
          {s.securityItems.map((item) => (
            <li key={item}>{item}</li>
          ))}
        </ul>
      </Panel>

      <Panel title={s.callsTitle} description={s.callsDescription}>
        {calls.isError && <ErrorState error={calls.error} />}
        {calls.data?.length === 0 && (
          <EmptyState icon={Plug} title={s.callsEmpty} description={s.callsEmptyHint} />
        )}
        {calls.data && calls.data.length > 0 && (
          <ul className="flex flex-col gap-1" aria-label={s.callsTitle}>
            {calls.data.map((op) => (
              <CallRow key={op.id} op={op} />
            ))}
          </ul>
        )}
      </Panel>
    </>
  );
}

function detailField(op: OperationRecord, key: string): unknown {
  const d = op.detail;
  if (typeof d !== "object" || d === null || Array.isArray(d)) return undefined;
  return (d as Record<string, unknown>)[key];
}

function CallRow({ op }: { op: OperationRecord }) {
  const tool = detailField(op, "tool");
  const isError = detailField(op, "isError") === true;
  const client = detailField(op, "clientName");
  const label =
    op.kind === "mcp.session"
      ? s.session(typeof client === "string" ? client : t.common.unknown)
      : typeof tool === "string"
        ? tool
        : op.kind;
  return (
    <li className="flex items-center gap-3 text-sm">
      <span className="w-28 shrink-0 text-xs text-fg-muted" title={formatDateTime(op.startedAt)}>
        {formatRelative(op.startedAt)}
      </span>
      <span className="min-w-0 flex-1 truncate font-mono text-xs">{label}</span>
      {op.kind === "mcp.tool" && (
        <Badge tone={isError ? "danger" : "success"}>{isError ? s.callError : s.callOk}</Badge>
      )}
      <span className="w-16 shrink-0 text-right text-xs text-fg-muted tabular-nums">
        {op.durationMs !== null ? formatDuration(op.durationMs) : ""}
      </span>
    </li>
  );
}
