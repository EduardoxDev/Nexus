/** Hooks TanStack Query sobre os comandos do core. Chaves centralizadas para invalidação. */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import type { OperationQuery, SettingKey } from "@/lib/bindings";
import { call } from "@/lib/ipc";

export const queryKeys = {
  startup: ["startup"] as const,
  appInfo: ["appInfo"] as const,
  settings: ["settings"] as const,
  diagnostics: ["diagnostics"] as const,
  operations: (query: OperationQuery) => ["operations", query] as const,
  operationsAll: ["operations"] as const,
};

export function useStartupStatus() {
  return useQuery({
    queryKey: queryKeys.startup,
    queryFn: () => call("get_startup_status"),
    staleTime: Infinity,
  });
}

export function useAppInfo() {
  return useQuery({
    queryKey: queryKeys.appInfo,
    queryFn: () => call("get_app_info"),
    staleTime: Infinity,
  });
}

export function useSettings() {
  return useQuery({ queryKey: queryKeys.settings, queryFn: () => call("get_settings") });
}

export function useSetSetting() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ key, value }: { key: SettingKey; value: unknown }) =>
      call("set_setting", { key, value }),
    onSuccess: (settings) => {
      client.setQueryData(queryKeys.settings, settings);
      void client.invalidateQueries({ queryKey: queryKeys.operationsAll });
    },
  });
}

export function useDiagnostics() {
  return useQuery({ queryKey: queryKeys.diagnostics, queryFn: () => call("get_diagnostics") });
}

export function useOperations(query: OperationQuery, refetchIntervalMs?: number) {
  return useQuery({
    queryKey: queryKeys.operations(query),
    queryFn: () => call("list_operations", { query }),
    ...(refetchIntervalMs === undefined ? {} : { refetchInterval: refetchIntervalMs }),
  });
}

export function useIntegrityCheck() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: () => call("run_integrity_check"),
    onSettled: () => {
      void client.invalidateQueries({ queryKey: queryKeys.diagnostics });
      void client.invalidateQueries({ queryKey: queryKeys.operationsAll });
    },
  });
}

export function useRevealDataDir() {
  return useMutation({ mutationFn: () => call("reveal_data_dir") });
}

// ---- Projetos e indexação ----------------------------------------------------------------

export const projectKeys = {
  all: ["projects"] as const,
  list: ["projects", "list"] as const,
  overview: (id: number) => ["projects", "overview", id] as const,
  jobs: (id: number) => ["projects", "jobs", id] as const,
  skipped: (id: number) => ["projects", "skipped", id] as const,
  jobErrors: (jobId: string) => ["projects", "jobErrors", jobId] as const,
};

export function useProjects() {
  return useQuery({ queryKey: projectKeys.list, queryFn: () => call("list_projects") });
}

export function useProjectOverview(projectId: number | null) {
  return useQuery({
    queryKey: projectKeys.overview(projectId ?? -1),
    queryFn: () => call("get_project_overview", { projectId: projectId ?? -1 }),
    enabled: projectId !== null,
  });
}

export function useJobs(projectId: number, limit = 20) {
  return useQuery({
    queryKey: projectKeys.jobs(projectId),
    queryFn: () => call("list_jobs", { projectId, limit }),
  });
}

export function useJobErrors(jobId: string | null) {
  return useQuery({
    queryKey: projectKeys.jobErrors(jobId ?? ""),
    queryFn: () => call("get_job_errors", { jobId: jobId ?? "", limit: 200 }),
    enabled: jobId !== null,
  });
}

export function useSkippedFiles(projectId: number) {
  return useQuery({
    queryKey: projectKeys.skipped(projectId),
    queryFn: () => call("list_skipped_files", { projectId, limit: 300 }),
  });
}

/** Abre o seletor nativo e registra a pasta escolhida. `null` = usuário cancelou. */
export function useAddProject() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (path?: string) => {
      const chosen = path ?? (await call("pick_project_folder"));
      if (chosen === null) return null;
      return call("add_project", { path: chosen });
    },
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: projectKeys.all });
      void client.invalidateQueries({ queryKey: queryKeys.operationsAll });
    },
  });
}

export function useRemoveProject() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (projectId: number) => call("remove_project", { projectId }),
    onSettled: () => {
      void client.invalidateQueries({ queryKey: projectKeys.all });
      void client.invalidateQueries({ queryKey: queryKeys.operationsAll });
    },
  });
}

export function useIndexControl() {
  const start = useMutation({
    mutationFn: ({ projectId, mode }: { projectId: number; mode: "full" | "incremental" }) =>
      call("start_index", { projectId, mode }),
  });
  const pause = useMutation({
    mutationFn: (projectId: number) => call("pause_index", { projectId }),
  });
  const resume = useMutation({
    mutationFn: (projectId: number) => call("resume_index", { projectId }),
  });
  const cancel = useMutation({
    mutationFn: (projectId: number) => call("cancel_index", { projectId }),
  });
  return { start, pause, resume, cancel };
}

export function useRevealProjectDir() {
  return useMutation({
    mutationFn: (projectId: number) => call("reveal_project_dir", { projectId }),
  });
}

/** Consulta padrão de operações (todos os campos explícitos, como exige o contrato). */
export function operationQuery(partial: Partial<OperationQuery> = {}): OperationQuery {
  return { limit: null, kindPrefix: null, status: null, ...partial };
}
