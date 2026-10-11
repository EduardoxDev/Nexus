import { Inbox } from "lucide-react";
import { Fragment, useState } from "react";

import { EmptyState, ErrorState } from "@/components/feedback";
import { Panel } from "@/components/layout";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { jobDuration } from "@/features/projects/activity";
import type { IndexJob, JobStatus } from "@/lib/bindings";
import { formatDateTime, formatDuration, formatRelative } from "@/lib/format";
import { t } from "@/lib/i18n/pt-BR";
import { useJobErrors, useJobs } from "@/lib/queries";

const statusTone: Record<JobStatus, "neutral" | "success" | "danger" | "warning" | "info"> = {
  running: "info",
  paused: "info",
  succeeded: "success",
  failed: "danger",
  cancelled: "neutral",
  interrupted: "warning",
};

export function JobsPanel({ projectId }: { projectId: number }) {
  const jobs = useJobs(projectId);
  const [expanded, setExpanded] = useState<string | null>(null);
  const columns = t.projects.jobsColumns;
  return (
    <Panel title={t.projects.jobsTitle}>
      {jobs.isError ? (
        <ErrorState error={jobs.error} onRetry={() => void jobs.refetch()} />
      ) : !jobs.data ? (
        <Skeleton className="h-24" />
      ) : jobs.data.length === 0 ? (
        <EmptyState icon={Inbox} title={t.projects.jobsEmpty} />
      ) : (
        <table className="w-full border-collapse text-sm">
          <thead>
            <tr className="text-left text-xs text-fg-subtle">
              {columns.map((c) => (
                <th key={c} scope="col" className="pb-2 font-medium">
                  {c}
                </th>
              ))}
              <th scope="col" className="pb-2">
                <span className="sr-only">{t.projects.errorsColumn}</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {jobs.data.map((job) => (
              <Fragment key={job.id}>
                <JobRow
                  job={job}
                  expanded={expanded === job.id}
                  onToggle={() => setExpanded((cur) => (cur === job.id ? null : job.id))}
                />
                {expanded === job.id && (
                  <tr>
                    <td colSpan={6} className="pb-3">
                      <JobErrors jobId={job.id} />
                    </td>
                  </tr>
                )}
              </Fragment>
            ))}
          </tbody>
        </table>
      )}
    </Panel>
  );
}

function JobRow({
  job,
  expanded,
  onToggle,
}: {
  job: IndexJob;
  expanded: boolean;
  onToggle: () => void;
}) {
  const duration = jobDuration(job);
  return (
    <tr className="border-t border-border">
      <td className="py-1.5 pr-2 text-fg-muted" title={formatDateTime(job.startedAt)}>
        {formatRelative(job.startedAt)}
      </td>
      <td className="py-1.5 pr-2">
        {t.jobTrigger[job.trigger]} · {t.jobMode[job.mode]}
      </td>
      <td className="py-1.5 pr-2">
        <Badge tone={statusTone[job.status]}>{t.jobStatus[job.status]}</Badge>
      </td>
      <td className="py-1.5 pr-2 text-fg-muted">
        {t.projects.jobFiles(job.processed, job.discovered)}
      </td>
      <td className="py-1.5 pr-2 font-mono text-xs text-fg-muted">
        {duration === null ? "—" : formatDuration(duration)}
      </td>
      <td className="py-1.5 text-right">
        {(job.failed > 0 || job.errorMessage !== null) && (
          <Button size="sm" variant="ghost" onClick={onToggle} aria-expanded={expanded}>
            {expanded ? t.projects.hideErrors : t.projects.showErrors}
          </Button>
        )}
      </td>
    </tr>
  );
}

function JobErrors({ jobId }: { jobId: string }) {
  const errors = useJobErrors(jobId);
  if (errors.isError) return <ErrorState error={errors.error} />;
  if (!errors.data) return <Skeleton className="h-8" />;
  if (errors.data.length === 0) {
    return <p className="text-xs text-fg-muted">{t.projects.jobErrorsEmpty}</p>;
  }
  return (
    <ul className="flex flex-col gap-1 rounded-md border border-border bg-surface-2 p-2.5">
      {errors.data.map((e) => (
        <li key={`${e.relPath}-${e.at}-${e.code}`} className="text-xs">
          <span className="selectable font-mono text-fg">{e.relPath || "—"}</span>{" "}
          <Badge tone="danger">{e.code}</Badge> <span className="text-fg-muted">{e.message}</span>
        </li>
      ))}
    </ul>
  );
}
