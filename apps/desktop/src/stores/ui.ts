/**
 * Estado de interface (não é estado de servidor): rota atual com histórico, paleta e atalhos.
 * Dados vindos do core ficam no TanStack Query.
 */
import { create } from "zustand";

import { isAvailable, type RouteId } from "@/app/routes";

const HISTORY_LIMIT = 50;

interface UiState {
  route: RouteId;
  past: RouteId[];
  future: RouteId[];
  paletteOpen: boolean;
  shortcutsOpen: boolean;
  /** Projeto aberto na seção Projetos. */
  selectedProjectId: number | null;
  selectProject: (id: number | null) => void;
  /** Tarefa sugerida para o próximo Context Pack (ex.: vinda da Busca). */
  draftTask: string;
  setDraftTask: (task: string) => void;
  /** Arquivo a abrir na prévia da Busca (ex.: clicado no Mapa). */
  pendingFile: string | null;
  /** Vai para a Busca com a prévia do arquivo aberta. */
  openFile: (relPath: string) => void;
  /** Lê e limpa o arquivo pendente (a Busca chama ao abrir). */
  takePendingFile: () => string | null;
  /** Memória aberta na seção Memória (ex.: vinda de um alerta de atualidade). */
  selectedMemoryId: string | null;
  selectMemory: (id: string | null) => void;
  /** Navega para uma seção disponível. Retorna `false` se a seção ainda não existe. */
  navigate: (route: RouteId) => boolean;
  back: () => void;
  forward: () => void;
  setPaletteOpen: (open: boolean) => void;
  setShortcutsOpen: (open: boolean) => void;
}

export const useUi = create<UiState>()((set, get) => ({
  route: "overview",
  past: [],
  future: [],
  paletteOpen: false,
  shortcutsOpen: false,
  selectedProjectId: null,
  selectProject: (selectedProjectId) => set({ selectedProjectId }),
  draftTask: "",
  setDraftTask: (draftTask) => set({ draftTask }),
  selectedMemoryId: null,
  selectMemory: (selectedMemoryId) => set({ selectedMemoryId }),
  pendingFile: null,
  openFile: (pendingFile) => {
    set({ pendingFile });
    get().navigate("search");
  },
  takePendingFile: () => {
    const file = get().pendingFile;
    if (file !== null) set({ pendingFile: null });
    return file;
  },
  navigate: (route) => {
    if (!isAvailable(route)) return false;
    const current = get().route;
    if (current === route) return true;
    set((s) => ({ route, past: [...s.past, current].slice(-HISTORY_LIMIT), future: [] }));
    return true;
  },
  back: () => {
    const { past, route, future } = get();
    const previous = past.at(-1);
    if (previous === undefined) return;
    set({ route: previous, past: past.slice(0, -1), future: [route, ...future] });
  },
  forward: () => {
    const { past, route, future } = get();
    const next = future[0];
    if (next === undefined) return;
    set({ route: next, past: [...past, route], future: future.slice(1) });
  },
  setPaletteOpen: (paletteOpen) => set({ paletteOpen }),
  setShortcutsOpen: (shortcutsOpen) => set({ shortcutsOpen }),
}));

/** Restaura o estado inicial (usado em testes). */
export function resetUi(): void {
  useUi.setState({
    route: "overview",
    past: [],
    future: [],
    paletteOpen: false,
    shortcutsOpen: false,
    selectedProjectId: null,
    draftTask: "",
    pendingFile: null,
  });
}
