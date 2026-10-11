import type { IndexJob, ProjectSummary } from "@/lib/bindings";
import { formatInteger, formatRelative } from "@/lib/format";
import { t } from "@/lib/i18n/pt-BR";
import type { ProjectActivity } from "@/stores/indexing";

/** Fração concluída (só existe na fase de indexação, quando o total é conhecido). */
export function jobFraction(job: IndexJob | null): number | null {
  if (!job || job.phase !== "indexing" || job.toProcess === 0) return null;
  return job.processed / job.toProcess;
}

export function jobProgressLabel(job: IndexJob | null, paused: boolean): string {
  if (paused) return t.projects.paused;
  if (!job || job.phase === "scanning") return t.projects.scanning;
  if (job.phase === "finalizing" || job.phase === "done") return t.projects.finalizing;
  return t.projects.indexingProgress(formatInteger(job.processed), formatInteger(job.toProcess));
}

/** Linha de estado curta para listas. */
export function activityLine(summary: ProjectSummary, activity: ProjectActivity): string {
  if (activity.kind === "running") return jobProgressLabel(activity.job, activity.paused);
  if (activity.kind === "queued") return t.projects.queued(activity.position);
  if (!summary.rootExists) return t.projects.rootMissing;
  if (summary.project.lastIndexedAt === null) return t.projects.neverIndexed;
  return t.projects.indexedAgo(
    formatInteger(summary.counts.indexed),
    formatRelative(summary.project.lastIndexedAt),
  );
}

export function jobDuration(job: IndexJob): number | null {
  return job.finishedAt === null ? null : job.finishedAt - job.startedAt;
}
