/** Hooks TanStack Query sobre os comandos do core. Chaves centralizadas para invalidação. */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import type {
  ContextPackRequest,
  ExportFormat,
  ExportOptions,
  IssueQuery,
  MemoryQuery,
  MemoryUpdate,
  NewMemory,
  OperationQuery,
  SearchRequest,
  SettingKey,
  Transition,
} from "@/lib/bindings";
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

// ---- Busca e Context Packs --------------------------------------------------------------

export const contextKeys = {
  search: (req: SearchRequest | null) => ["search", req] as const,
  fileContext: (projectId: number, relPath: string, start: number, end: number) =>
    ["fileContext", projectId, relPath, start, end] as const,
  packs: (projectId: number) => ["packs", projectId] as const,
  pack: (id: string) => ["pack", id] as const,
  packsAll: ["packs"] as const,
};

/** Executa a busca quando há uma requisição (enviada pelo formulário). */
export function useSearch(request: SearchRequest | null) {
  return useQuery({
    queryKey: contextKeys.search(request),
    queryFn: () => {
      if (!request) throw new Error("sem consulta");
      return call("search_project", { request });
    },
    enabled: request !== null,
    staleTime: 0,
  });
}

export function useFileContext(
  projectId: number,
  relPath: string | null,
  startLine: number,
  endLine: number,
) {
  return useQuery({
    queryKey: contextKeys.fileContext(projectId, relPath ?? "", startLine, endLine),
    queryFn: () =>
      call("get_file_context", { projectId, relPath: relPath ?? "", startLine, endLine }),
    enabled: relPath !== null,
  });
}

export function useContextPacks(projectId: number | null) {
  return useQuery({
    queryKey: contextKeys.packs(projectId ?? -1),
    queryFn: () => call("list_context_packs", { projectId: projectId ?? -1, limit: 50 }),
    enabled: projectId !== null,
  });
}

export function useContextPack(packId: string | null) {
  return useQuery({
    queryKey: contextKeys.pack(packId ?? ""),
    queryFn: () => call("get_context_pack", { packId: packId ?? "" }),
    enabled: packId !== null,
  });
}

export function useCompilePack() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (request: ContextPackRequest) => call("compile_context_pack", { request }),
    onSuccess: (pack) => {
      client.setQueryData(contextKeys.pack(pack.plan.id), pack);
      void client.invalidateQueries({ queryKey: contextKeys.packsAll });
      void client.invalidateQueries({ queryKey: queryKeys.operationsAll });
    },
  });
}

export function useDeletePack() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (packId: string) => call("delete_context_pack", { packId }),
    onSuccess: () => void client.invalidateQueries({ queryKey: contextKeys.packsAll }),
  });
}

export function usePackExport() {
  const render = useMutation({
    mutationFn: (args: { packId: string; format: ExportFormat; options: ExportOptions }) =>
      call("render_context_pack", args),
  });
  const save = useMutation({
    mutationFn: (args: { packId: string; format: ExportFormat; options: ExportOptions }) =>
      call("save_context_pack", args),
  });
  return { render, save };
}

/** Consulta padrão de operações (todos os campos explícitos, como exige o contrato). */
export function operationQuery(partial: Partial<OperationQuery> = {}): OperationQuery {
  return { limit: null, kindPrefix: null, status: null, ...partial };
}

// ---- Memória e atualidade ---------------------------------------------------------------

export const memoryKeys = {
  all: ["memory"] as const,
  list: (query: MemoryQuery) => ["memory", "list", query] as const,
  detail: (id: string) => ["memory", "detail", id] as const,
  counts: (projectId: number) => ["memory", "counts", projectId] as const,
};

export const freshnessKeys = {
  all: ["freshness"] as const,
  issues: (query: IssueQuery) => ["freshness", "issues", query] as const,
  summary: (projectId: number) => ["freshness", "summary", projectId] as const,
};

/** Mudanças na memória reavaliam alertas no core: as duas áreas e os packs são invalidados. */
function invalidateMemory(client: ReturnType<typeof useQueryClient>) {
  void client.invalidateQueries({ queryKey: memoryKeys.all });
  void client.invalidateQueries({ queryKey: freshnessKeys.all });
  void client.invalidateQueries({ queryKey: ["pack"] });
  void client.invalidateQueries({ queryKey: queryKeys.operationsAll });
}

export function useMemories(query: MemoryQuery | null) {
  return useQuery({
    queryKey: memoryKeys.list(query ?? { projectId: -1 }),
    queryFn: () => call("list_memories", { query: query ?? { projectId: -1 } }),
    enabled: query !== null,
  });
}

export function useMemory(memoryId: string | null) {
  return useQuery({
    queryKey: memoryKeys.detail(memoryId ?? ""),
    queryFn: () => call("get_memory", { memoryId: memoryId ?? "" }),
    enabled: memoryId !== null,
  });
}

export function useMemoryCounts(projectId: number | null) {
  return useQuery({
    queryKey: memoryKeys.counts(projectId ?? -1),
    queryFn: () => call("get_memory_counts", { projectId: projectId ?? -1 }),
    enabled: projectId !== null,
  });
}

export function useMemoryMutations() {
  const client = useQueryClient();
  const onSuccess = () => invalidateMemory(client);
  const create = useMutation({
    mutationFn: (request: NewMemory) => call("create_memory", { request }),
    onSuccess,
  });
  const update = useMutation({
    mutationFn: (args: { memoryId: string; request: MemoryUpdate }) => call("update_memory", args),
    onSuccess,
  });
  const transition = useMutation({
    mutationFn: (args: {
      memoryId: string;
      transition: Transition;
      expectedRevision: number | null;
      note: string | null;
    }) => call("transition_memory", args),
    onSuccess,
  });
  const reviewRefs = useMutation({
    mutationFn: (args: { memoryId: string; expectedRevision: number }) =>
      call("review_memory_refs", args),
    onSuccess,
  });
  const importAdrs = useMutation({
    mutationFn: (projectId: number) => call("import_adrs", { projectId }),
    onSuccess,
  });
  return { create, update, transition, reviewRefs, importAdrs };
}

export function useFreshnessIssues(query: IssueQuery | null) {
  return useQuery({
    queryKey: freshnessKeys.issues(query ?? { projectId: -1 }),
    queryFn: () => call("list_freshness_issues", { query: query ?? { projectId: -1 } }),
    enabled: query !== null,
  });
}

export function useFreshnessSummary(projectId: number | null) {
  return useQuery({
    queryKey: freshnessKeys.summary(projectId ?? -1),
    queryFn: () => call("get_freshness_summary", { projectId: projectId ?? -1 }),
    enabled: projectId !== null,
  });
}

export function useFreshnessMutations() {
  const client = useQueryClient();
  const onSuccess = () => {
    void client.invalidateQueries({ queryKey: freshnessKeys.all });
    void client.invalidateQueries({ queryKey: ["pack"] });
    void client.invalidateQueries({ queryKey: queryKeys.operationsAll });
  };
  const scan = useMutation({
    mutationFn: (projectId: number) => call("scan_freshness", { projectId }),
    onSuccess,
  });
  const dismiss = useMutation({
    mutationFn: (args: { issueId: number; dismissed: boolean; note: string | null }) =>
      call("set_issue_dismissed", args),
    onSuccess,
  });
  return { scan, dismiss };
}

export function useIntegrationInfo() {
  return useQuery({
    queryKey: ["integrationInfo"] as const,
    queryFn: () => call("get_integration_info"),
    staleTime: Infinity,
  });
}

// ---- Análise de código -------------------------------------------------------------------

export function useAnalysisOverview(projectId: number) {
  return useQuery({
    queryKey: [...projectKeys.all, "analysis", projectId] as const,
    queryFn: () => call("get_analysis_overview", { projectId }),
  });
}

export function useFileAnalysis(projectId: number, relPath: string) {
  return useQuery({
    queryKey: [...projectKeys.all, "fileAnalysis", projectId, relPath] as const,
    queryFn: () => call("get_file_analysis", { projectId, relPath }),
  });
}
