import { FolderGit2, FolderPlus, TriangleAlert } from "lucide-react";
import { useEffect } from "react";

import { EmptyState, ErrorState } from "@/components/feedback";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { Skeleton } from "@/components/ui/skeleton";
import { activityLine, jobFraction, jobProgressLabel } from "@/features/projects/activity";
import { ProjectDetail } from "@/features/projects/ProjectDetail";
import type { ProjectSummary } from "@/lib/bindings";
import { cn } from "@/lib/cn";
import { t } from "@/lib/i18n/pt-BR";
import { useAddProject, useProjects } from "@/lib/queries";
import { useProjectActivity } from "@/stores/indexing";
import { useUi } from "@/stores/ui";

export function ProjectsPage() {
  const projects = useProjects();
  const selected = useUi((s) => s.selectedProjectId);
  const selectProject = useUi((s) => s.selectProject);
  const add = useAddProject();

  // Seleciona o primeiro projeto quando nada está selecionado (ou o selecionado sumiu).
  useEffect(() => {
    const list = projects.data;
    if (!list) return;
    if (list.length === 0) {
      if (selected !== null) selectProject(null);
      return;
    }
    if (selected === null || !list.some((p) => p.project.id === selected)) {
      selectProject(list[0]?.project.id ?? null);
    }
  }, [projects.data, selected, selectProject]);

  const addButton = (
    <Button
      variant="primary"
      onClick={() =>
        add.mutate(undefined, {
          onSuccess: (summary) => {
            if (summary) selectProject(summary.project.id);
          },
        })
      }
      disabled={add.isPending}
    >
      <FolderPlus aria-hidden="true" />
      {add.isPending ? t.projects.adding : t.projects.add}
    </Button>
  );

  if (projects.isError) {
    return (
      <div className="p-6">
        <ErrorState error={projects.error} onRetry={() => void projects.refetch()} />
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-0">
      <aside
        aria-label={t.projects.listLabel}
        className="flex w-72 shrink-0 flex-col border-r border-border bg-surface-sidebar"
      >
        <div className="flex items-center justify-between gap-2 border-b border-border px-3 py-2.5">
          <h1 className="font-display text-lg font-semibold tracking-tight">
            {t.nav.projects.label}
          </h1>
          {projects.data && projects.data.length > 0 && (
            <Button
              size="sm"
              variant="secondary"
              onClick={() =>
                add.mutate(undefined, {
                  onSuccess: (summary) => {
                    if (summary) selectProject(summary.project.id);
                  },
                })
              }
              disabled={add.isPending}
              aria-label={t.projects.add}
            >
              <FolderPlus aria-hidden="true" />
              {t.projects.add}
            </Button>
          )}
        </div>
        {add.isError && <ErrorState className="m-3" error={add.error} />}
        <div className="flex-1 overflow-y-auto p-1.5">
          {!projects.data ? (
            <div className="flex flex-col gap-2 p-2" aria-busy="true">
              <Skeleton className="h-12" />
              <Skeleton className="h-12" />
            </div>
          ) : (
            <ul className="flex flex-col gap-0.5">
              {projects.data.map((summary) => (
                <li key={summary.project.id}>
                  <ProjectRow
                    summary={summary}
                    selected={summary.project.id === selected}
                    onSelect={() => selectProject(summary.project.id)}
                  />
                </li>
              ))}
            </ul>
          )}
        </div>
      </aside>

      <div className="min-w-0 flex-1 overflow-y-auto">
        {projects.data?.length === 0 ? (
          <div className="flex h-full items-center justify-center">
            <EmptyState
              icon={FolderGit2}
              title={t.projects.emptyTitle}
              description={t.projects.emptyBody}
              action={addButton}
            />
          </div>
        ) : selected !== null ? (
          <ProjectDetail key={selected} projectId={selected} />
        ) : (
          <p className="p-6 text-sm text-fg-muted">{t.projects.selectHint}</p>
        )}
      </div>
    </div>
  );
}

function ProjectRow({
  summary,
  selected,
  onSelect,
}: {
  summary: ProjectSummary;
  selected: boolean;
  onSelect: () => void;
}) {
  const activity = useProjectActivity(summary.project.id);
  const running = activity.kind === "running";
  return (
    <button
      type="button"
      onClick={onSelect}
      aria-current={selected ? "true" : undefined}
      className={cn(
        "flex w-full flex-col gap-1 rounded-md px-2.5 py-2 text-left transition-colors",
        selected ? "bg-accent-subtle" : "hover:bg-surface-2",
      )}
    >
      <span className="flex items-center gap-1.5">
        <span className="truncate text-sm font-medium text-fg">{summary.project.name}</span>
        {!summary.rootExists && (
          <TriangleAlert
            className="size-3.5 shrink-0 text-warning"
            aria-label={t.projects.rootMissing}
          />
        )}
      </span>
      <span className="truncate text-xs text-fg-muted">{activityLine(summary, activity)}</span>
      {running && (
        <Progress
          value={jobFraction(activity.job)}
          label={jobProgressLabel(activity.job, activity.paused)}
        />
      )}
    </button>
  );
}
