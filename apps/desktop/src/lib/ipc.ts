/**
 * Camada IPC tipada. Cada comando do core tem argumentos e resultado declarados aqui, usando os
 * tipos gerados a partir do Rust (src/lib/bindings). Nenhum outro módulo chama `invoke` diretamente.
 */
import { invoke } from "@tauri-apps/api/core";

import type {
  AppInfo,
  DiagnosticsReport,
  ErrorCode,
  ErrorInfo,
  IntegrityReport,
  OperationQuery,
  OperationRecord,
  SettingKey,
  Settings,
  StartupStatus,
} from "@/lib/bindings";

export interface Commands {
  get_startup_status: { args: undefined; result: StartupStatus };
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
