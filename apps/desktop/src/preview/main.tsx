/**
 * Prévia do app no navegador, só em desenvolvimento (`/preview.html`): o IPC do Tauri é simulado
 * com dados de exemplo para trabalhar no visual sem abrir o app. Não entra no build.
 */
import { mockIPC } from "@tauri-apps/api/mocks";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "@/app/App";
import { Providers } from "@/app/providers";
import type { MapRequest, Settings } from "@/lib/bindings";
import "@fontsource-variable/inter/wght.css";
import "@/styles/globals.css";

import * as data from "./data";

let settings: Settings = { ...data.settings };
const projectId = (args: Record<string, unknown>) => Number(args.projectId ?? 1);

const handlers: Record<string, (args: Record<string, unknown>) => unknown> = {
  get_startup_status: () => data.startup,
  get_app_info: () => data.appInfo,
  get_settings: () => settings,
  set_setting: (args) => {
    const key = String(args.key);
    const map: Record<string, keyof Settings> = {
      "ui.theme": "theme",
      "ui.reduced_motion": "reducedMotion",
      "ui.sidebar_collapsed": "sidebarCollapsed",
      "indexing.threads": "indexingThreads",
      "indexing.max_file_size_kb": "maxFileSizeKb",
      "indexing.watch": "watchProjects",
      "updates.check": "updateCheck",
    };
    const field = map[key];
    if (field) settings = { ...settings, [field]: args.value };
    return settings;
  },
  get_diagnostics: () => data.diagnostics,
  list_operations: () => data.operations,
  run_integrity_check: () => ({ ok: true, messages: ["ok"], durationMs: 42, opId: "op-check" }),
  list_projects: () => data.projects,
  get_project_overview: (args) => data.overview(projectId(args)),
  get_indexing_queue: () => ({ running: null, paused: false, queued: [] }),
  list_jobs: (args) => data.jobs(projectId(args)),
  get_job_errors: () => [],
  list_skipped_files: () => [],
  touch_project: () => null,
  search_project: () => data.search,
  list_context_packs: () => data.packs,
  list_memories: () => data.memories,
  get_memory_counts: () => data.memoryCounts,
  list_freshness_issues: () => data.issues,
  get_freshness_summary: () => data.freshnessSummary,
  get_integration_info: () => data.integration,
  check_for_update: () => ({ status: "upToDate", currentVersion: "0.2.0" }),
  get_analysis_overview: () => data.analysis,
  preview_memory_export: () => data.exportPreview,
  export_memory: () => ({ directory: ".nexus/memory", created: 2, updated: 1, unchanged: 1 }),
  get_project_map: (args) => data.projectMap(args.request as MapRequest),
};

mockIPC(
  (cmd, payload) => {
    const handler = handlers[cmd];
    if (!handler) {
      console.warn(`[prévia] comando sem dados de exemplo: ${cmd}`);
      return null;
    }
    return handler((payload ?? {}) as Record<string, unknown>);
  },
  { shouldMockEvents: true },
);

const root = document.getElementById("root");
if (!root) throw new Error("#root ausente");
createRoot(root).render(
  <StrictMode>
    <Providers>
      <App />
    </Providers>
  </StrictMode>,
);
