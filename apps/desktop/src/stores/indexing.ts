/**
 * Estado ao vivo da indexação, alimentado pelos eventos do core (`nexus://…`).
 * Fica fora do TanStack Query porque não é buscado: é empurrado pelo backend.
 */
import { useMemo } from "react";
import { create } from "zustand";

import type { IndexingQueue, IndexJob } from "@/lib/bindings";

interface IndexingState {
  /** Último estado conhecido do job ativo de cada projeto. */
  live: Record<number, IndexJob>;
  queue: IndexingQueue;
  applyProgress: (job: IndexJob) => void;
  applyFinished: (job: IndexJob) => void;
  setQueue: (queue: IndexingQueue) => void;
}

const EMPTY_QUEUE: IndexingQueue = { running: null, paused: false, queued: [] };

export const useIndexing = create<IndexingState>()((set) => ({
  live: {},
  queue: EMPTY_QUEUE,
  applyProgress: (job) => set((s) => ({ live: { ...s.live, [job.projectId]: job } })),
  applyFinished: (job) =>
    set((s) => ({
      live: Object.fromEntries(
        Object.entries(s.live).filter(([id]) => Number(id) !== job.projectId),
      ),
    })),
  setQueue: (queue) => set({ queue }),
}));

export function resetIndexing(): void {
  useIndexing.setState({ live: {}, queue: EMPTY_QUEUE });
}

/** Estado de indexação de um projeto para exibição. */
export type ProjectActivity =
  | { kind: "running"; job: IndexJob | null; paused: boolean }
  | { kind: "queued"; position: number }
  | { kind: "idle" };

export function activityOf(
  projectId: number,
  state: Pick<IndexingState, "live" | "queue">,
): ProjectActivity {
  if (state.queue.running === projectId) {
    return { kind: "running", job: state.live[projectId] ?? null, paused: state.queue.paused };
  }
  const position = state.queue.queued.indexOf(projectId);
  if (position >= 0) return { kind: "queued", position: position + 1 };
  return { kind: "idle" };
}

/** Atividade de um projeto, com seletores estáveis (evita re-render em loop). */
export function useProjectActivity(projectId: number): ProjectActivity {
  const job = useIndexing((s) => s.live[projectId]);
  const queue = useIndexing((s) => s.queue);
  return useMemo(
    () => activityOf(projectId, { live: job ? { [projectId]: job } : {}, queue }),
    [projectId, job, queue],
  );
}
