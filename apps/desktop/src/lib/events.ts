/** Assinatura dos eventos tipados do core. Montado uma vez no shell. */
import { useQueryClient } from "@tanstack/react-query";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect } from "react";

import type { IndexingQueue, IndexJob } from "@/lib/bindings";
import { call } from "@/lib/ipc";
import { projectKeys, queryKeys } from "@/lib/queries";
import { useIndexing } from "@/stores/indexing";

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
          void client.invalidateQueries({ queryKey: projectKeys.all });
          void client.invalidateQueries({ queryKey: queryKeys.operationsAll });
          void client.invalidateQueries({ queryKey: queryKeys.diagnostics });
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
