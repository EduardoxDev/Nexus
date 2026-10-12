/** Assinatura dos eventos tipados do core. Montado uma vez no shell. */
import { useQueryClient } from "@tanstack/react-query";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect } from "react";

import type { IndexingQueue, IndexJob, ProjectSummary } from "@/lib/bindings";
import { t } from "@/lib/i18n/pt-BR";
import { call } from "@/lib/ipc";
import { freshnessKeys, memoryKeys, projectKeys, queryKeys } from "@/lib/queries";
import { useIndexing } from "@/stores/indexing";
import { useToasts } from "@/stores/toast";

/**
 * Avisa o fim de uma indexação: falhas sempre; sucesso só quando o usuário pediu (as
 * atualizações automáticas do monitor de arquivos e da reconciliação ficam em silêncio).
 */
export function notifyJobFinished(job: IndexJob, projectName: string): void {
  const { push } = useToasts.getState();
  if (job.status === "failed") {
    push({
      tone: "danger",
      title: t.toast.indexFailed(projectName),
      ...(job.errorMessage ? { body: job.errorMessage } : {}),
      action: { label: t.toast.details, route: "projects", projectId: job.projectId },
    });
  } else if (job.status === "succeeded" && (job.trigger === "user" || job.trigger === "resume")) {
    push({
      tone: "success",
      title: t.toast.indexed(projectName),
      body: t.toast.indexedBody(job.processed, job.unchanged, job.failed),
      action: { label: t.toast.search, route: "search", projectId: job.projectId },
    });
  }
}

export const EVENTS = {
  progress: "nexus://job-progress",
  finished: "nexus://job-finished",
  queue: "nexus://queue-changed",
} as const;

export function useIndexingEvents(): void {
  const client = useQueryClient();

  useEffect(() => {
    let disposed = false;
    const unlisteners: UnlistenFn[] = [];
    const { applyProgress, applyFinished, setQueue } = useIndexing.getState();

    const subscribe = async () => {
      const subs = await Promise.all([
        listen<IndexJob>(EVENTS.progress, (e) => {
          applyProgress(e.payload);
        }),
        listen<IndexJob>(EVENTS.finished, (e) => {
          applyFinished(e.payload);
          const projects = client.getQueryData<ProjectSummary[]>(projectKeys.list);
          const name =
            projects?.find((p) => p.project.id === e.payload.projectId)?.project.name ??
            `#${e.payload.projectId}`;
          notifyJobFinished(e.payload, name);
          void client.invalidateQueries({ queryKey: projectKeys.all });
          void client.invalidateQueries({ queryKey: queryKeys.operationsAll });
          void client.invalidateQueries({ queryKey: queryKeys.diagnostics });
          // O core reavalia a atualidade depois de cada indexação com mudanças.
          void client.invalidateQueries({ queryKey: freshnessKeys.all });
          void client.invalidateQueries({ queryKey: memoryKeys.all });
        }),
        listen<IndexingQueue>(EVENTS.queue, (e) => {
          setQueue(e.payload);
        }),
      ]);
      if (disposed) subs.forEach((u) => u());
      else unlisteners.push(...subs);
      // Estado inicial (a fila pode ter começado antes da interface).
      try {
        if (!disposed) setQueue(await call("get_indexing_queue"));
      } catch {
        // indisponível (ex.: banco não abriu) — a interface mostra o erro em outro lugar
      }
    };
    void subscribe();
    return () => {
      disposed = true;
      unlisteners.forEach((u) => u());
    };
  }, [client]);
}
