/**
 * Notificações discretas (toasts) para resultados que acontecem fora da tela atual, como o fim
 * de uma indexação. Somem sozinhas; erros ficam mais tempo.
 */
import { create } from "zustand";

import type { RouteId } from "@/app/routes";

export type ToastTone = "success" | "danger" | "info";

export interface Toast {
  id: number;
  tone: ToastTone;
  title: string;
  body?: string;
  /** Botão opcional que leva a uma seção. */
  action?: { label: string; route: RouteId; projectId?: number };
}

interface ToastState {
  toasts: Toast[];
  push: (toast: Omit<Toast, "id">) => number;
  dismiss: (id: number) => void;
}

const MAX_VISIBLE = 4;
const DURATION_MS: Record<ToastTone, number> = { success: 6_000, info: 6_000, danger: 12_000 };
let nextId = 1;

export const useToasts = create<ToastState>()((set, get) => ({
  toasts: [],
  push: (toast) => {
    const id = nextId++;
    set((s) => ({ toasts: [...s.toasts, { ...toast, id }].slice(-MAX_VISIBLE) }));
    window.setTimeout(() => get().dismiss(id), DURATION_MS[toast.tone]);
    return id;
  },
  dismiss: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}));

/** Limpa as notificações (testes). */
export function resetToasts(): void {
  useToasts.setState({ toasts: [] });
}
