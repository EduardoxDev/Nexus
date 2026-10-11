/**
 * Utilitários de teste: respostas de IPC simuladas (somente em testes) e renderização do app.
 */
import { mockIPC } from "@tauri-apps/api/mocks";
import { render } from "@testing-library/react";

import { App } from "@/app/App";
import { createQueryClient, Providers } from "@/app/providers";
import type {
  AppInfo,
  DiagnosticsReport,
  ErrorInfo,
  OperationRecord,
  Settings,
  StartupStatus,
} from "@/lib/bindings";
import type { CommandName } from "@/lib/ipc";

export const startupOk: StartupStatus = {
  ready: true,
  dataDir: "C:\\Users\\teste\\AppData\\Local\\Nexus",
  error: null,
  schemaVersion: 1,
  appliedMigrations: [],
  backupCreated: false,
  recoveredOperations: 0,
};

export const defaultSettings: Settings = {
  theme: "dark",
  reducedMotion: "system",
  sidebarCollapsed: false,
  indexingThreads: 0,
  maxFileSizeKb: 1024,
  watchProjects: true,
};

export const appInfo: AppInfo = {
  appVersion: "0.1.0",
  coreVersion: "0.1.0",
  buildProfile: "debug",
  os: "windows",
  arch: "x86_64",
  webviewVersion: "154.0.0.0",
  tauriVersion: "2.12.3",
};

export const operation = (over: Partial<OperationRecord> = {}): OperationRecord => ({
  id: "01a128c9-0000-7000-8000-000000000001",
  kind: "core.open",
  projectId: null,
  status: "succeeded",
  startedAt: Date.now() - 60_000,
  heartbeatAt: Date.now() - 60_000,
  finishedAt: Date.now() - 59_990,
  durationMs: 10,
  detail: { toVersion: 1 },
  errorCode: null,
  errorMessage: null,
  ...over,
});

export const diagnostics: DiagnosticsReport = {
  coreVersion: "0.1.0",
  sqliteVersion: "3.53.2",
  fts5Available: true,
  dataDir: startupOk.dataDir!,
  database: { path: "nexus.db", exists: true, sizeBytes: 4096 },
  wal: { path: "nexus.db-wal", exists: true, sizeBytes: 8192 },
  logsDir: "logs",
  logsSizeBytes: 1024,
  logFiles: 1,
  backupsDir: "backups",
  backups: [],
  schemaVersion: 1,
  latestSchemaVersion: 1,
  migrations: [
    { version: 1, name: "foundation", checksum: "bbc25775ace5abcdef", appliedAt: 0, durationMs: 1 },
  ],
  operationsTotal: 2,
  lastIntegrityCheck: null,
};

export type Handler = (args: Record<string, unknown>) => unknown;

export class IpcError extends Error {
  constructor(readonly info: ErrorInfo) {
    super(info.message);
  }
}

/** Registra respostas por comando. Um `IpcError` lançado vira rejeição com o envelope do core. */
export function setupIpc(overrides: Partial<Record<CommandName, Handler>> = {}) {
  const calls: { cmd: string; args: Record<string, unknown> }[] = [];
  let settings = { ...defaultSettings };
  const handlers: Record<string, Handler> = {
    get_startup_status: () => startupOk,
    get_app_info: () => appInfo,
    get_settings: () => settings,
    set_setting: (args) => {
      const key = args.key as string;
      if (key === "ui.theme") settings = { ...settings, theme: args.value as Settings["theme"] };
      if (key === "ui.reduced_motion")
        settings = { ...settings, reducedMotion: args.value as Settings["reducedMotion"] };
      if (key === "ui.sidebar_collapsed")
        settings = { ...settings, sidebarCollapsed: args.value as boolean };
      return settings;
    },
    get_diagnostics: () => diagnostics,
    list_operations: () => [operation()],
    run_integrity_check: () => ({ ok: true, messages: ["ok"], durationMs: 3, opId: "op-check" }),
    reveal_data_dir: () => null,
    list_projects: () => [],
    get_indexing_queue: () => ({ running: null, paused: false, queued: [] }),
    ...overrides,
  };
  mockIPC(
    (cmd, payload) => {
      const args = (payload ?? {}) as Record<string, unknown>;
      calls.push({ cmd, args });
      const handler = handlers[cmd];
      if (!handler) throw new Error(`comando não simulado: ${cmd}`);
      try {
        return handler(args);
      } catch (e) {
        // O Tauri rejeita com o objeto serializado do erro (ErrorInfo), não com um Error.
        // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors
        if (e instanceof IpcError) return Promise.reject(e.info);
        throw e;
      }
    },
    { shouldMockEvents: true },
  );
  return { calls };
}

export function renderApp() {
  const client = createQueryClient();
  client.setDefaultOptions({ queries: { retry: false }, mutations: { retry: false } });
  return render(
    <Providers client={client}>
      <App />
    </Providers>,
  );
}
