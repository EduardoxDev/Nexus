/**
 * Camada IPC tipada. Cada comando do core tem argumentos e resultado declarados aqui, usando os
 * tipos gerados a partir do Rust (src/lib/bindings). Nenhum outro módulo chama `invoke` diretamente.
 */
import { invoke } from "@tauri-apps/api/core";

import type {
  AppInfo,
  ContextPack,
  ContextPackRequest,
  ContextPackSummary,
  ExportFormat,
  ExportOptions,
  FileContext,
  FreshnessIssue,
  FreshnessSummary,
  AnalysisOverview,
  FileAnalysisView,
  FileHistory,
  MapRequest,
  ProjectMap,
  ImportReport,
  IntegrationInfo,
  UpdateStatus,
  IssueQuery,
  Memory,
  MemoryCounts,
  MemoryDetail,
  MemoryQuery,
  MemoryUpdate,
  NewMemory,
  ScanReport,
  Transition,
  SearchRequest,
  SearchResponse,
  DiagnosticsReport,
  ErrorCode,
  ErrorInfo,
  IndexError,
  IndexingQueue,
  IndexJob,
  IndexMode,
  IntegrityReport,
  OperationQuery,
  OperationRecord,
  ProjectOverview,
  ProjectSummary,
  SettingKey,
  Settings,
  SkippedFile,
  StartupStatus,
} from "@/lib/bindings";

type ProjectArgs = { projectId: number };

type PackArgs = { packId: string };
type ExportArgs = PackArgs & { format: ExportFormat; options: ExportOptions };

type MemoryArgs = { memoryId: string };

export interface Commands {
  list_memories: { args: { query: MemoryQuery }; result: Memory[] };
  get_memory: { args: MemoryArgs; result: MemoryDetail };
  get_memory_counts: { args: ProjectArgs; result: MemoryCounts };
  create_memory: { args: { request: NewMemory }; result: Memory };
  update_memory: { args: MemoryArgs & { request: MemoryUpdate }; result: Memory };
  transition_memory: {
    args: MemoryArgs & {
      transition: Transition;
      expectedRevision: number | null;
      note: string | null;
    };
    result: Memory;
  };
  review_memory_refs: { args: MemoryArgs & { expectedRevision: number }; result: Memory };
  import_adrs: { args: ProjectArgs; result: ImportReport };
  list_freshness_issues: { args: { query: IssueQuery }; result: FreshnessIssue[] };
  get_freshness_summary: { args: ProjectArgs; result: FreshnessSummary };
  scan_freshness: { args: ProjectArgs; result: ScanReport };
  set_issue_dismissed: {
    args: { issueId: number; dismissed: boolean; note: string | null };
    result: FreshnessIssue;
  };
  search_project: { args: { request: SearchRequest }; result: SearchResponse };
  get_file_context: {
    args: ProjectArgs & { relPath: string; startLine: number | null; endLine: number | null };
    result: FileContext;
  };
  compile_context_pack: { args: { request: ContextPackRequest }; result: ContextPack };
  list_context_packs: { args: ProjectArgs & { limit: number }; result: ContextPackSummary[] };
  get_context_pack: { args: PackArgs; result: ContextPack };
  delete_context_pack: { args: PackArgs; result: null };
  render_context_pack: { args: ExportArgs; result: string };
  save_context_pack: { args: ExportArgs; result: string | null };
  list_projects: { args: undefined; result: ProjectSummary[] };
  get_project_overview: { args: ProjectArgs; result: ProjectOverview };
  pick_project_folder: { args: undefined; result: string | null };
  add_project: { args: { path: string }; result: ProjectSummary };
  remove_project: { args: ProjectArgs; result: null };
  start_index: { args: ProjectArgs & { mode: IndexMode }; result: null };
  pause_index: { args: ProjectArgs; result: boolean };
  resume_index: { args: ProjectArgs; result: boolean };
  cancel_index: { args: ProjectArgs; result: boolean };
  get_indexing_queue: { args: undefined; result: IndexingQueue };
  list_jobs: { args: ProjectArgs & { limit: number }; result: IndexJob[] };
  get_job_errors: { args: { jobId: string; limit: number }; result: IndexError[] };
  list_skipped_files: { args: ProjectArgs & { limit: number }; result: SkippedFile[] };
  touch_project: { args: ProjectArgs; result: null };
  reveal_project_dir: { args: ProjectArgs; result: null };
  get_startup_status: { args: undefined; result: StartupStatus };
  get_integration_info: { args: undefined; result: IntegrationInfo };
  check_for_update: { args: { manual: boolean }; result: UpdateStatus };
  install_update: { args: undefined; result: null };
  get_analysis_overview: { args: ProjectArgs; result: AnalysisOverview };
  get_file_analysis: { args: ProjectArgs & { relPath: string }; result: FileAnalysisView };
  get_project_map: { args: { request: MapRequest }; result: ProjectMap };
  get_file_history: { args: ProjectArgs & { relPath: string }; result: FileHistory };
  get_app_info: { args: undefined; result: AppInfo };
  get_diagnostics: { args: undefined; result: DiagnosticsReport };
  run_integrity_check: { args: undefined; result: IntegrityReport };
  list_operations: { args: { query: OperationQuery }; result: OperationRecord[] };
  get_settings: { args: undefined; result: Settings };
  set_setting: { args: { key: SettingKey; value: unknown }; result: Settings };
  reveal_data_dir: { args: undefined; result: null };
}

export type CommandName = keyof Commands;

/** Erro de comando com código estável e `opId` para rastreio no Diagnóstico. */
export class NexusError extends Error {
  readonly code: ErrorCode;
  readonly opId: string | null;

  constructor(info: ErrorInfo) {
    super(info.message);
    this.name = "NexusError";
    this.code = info.code;
    this.opId = info.opId;
  }
}

function isErrorInfo(value: unknown): value is ErrorInfo {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    "message" in value &&
    typeof (value as { message: unknown }).message === "string"
  );
}

export function toNexusError(error: unknown): NexusError {
  if (error instanceof NexusError) return error;
  if (isErrorInfo(error)) return new NexusError(error);
  const message = error instanceof Error ? error.message : String(error);
  return new NexusError({ code: "internal", message, opId: null });
}

type ArgsParam<K extends CommandName> = Commands[K]["args"] extends undefined
  ? []
  : [args: Commands[K]["args"]];

export async function call<K extends CommandName>(
  command: K,
  ...args: ArgsParam<K>
): Promise<Commands[K]["result"]> {
  try {
    return await invoke<Commands[K]["result"]>(command, args[0]);
  } catch (error) {
    throw toNexusError(error);
  }
}
