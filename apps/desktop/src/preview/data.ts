/**
 * Dados de exemplo para a prévia do app no navegador (`/preview.html`, só em desenvolvimento).
 * Tipados com os contratos gerados do Rust: se o core mudar, a prévia não compila.
 */
import type {
  AnalysisOverview,
  AppInfo,
  ContextPack,
  ContextPackSummary,
  DiagnosticsReport,
  ExportPreview,
  FileContext,
  FreshnessIssue,
  FreshnessSummary,
  IndexJob,
  IntegrationInfo,
  MapNode,
  MapRequest,
  Memory,
  MemoryCounts,
  OperationRecord,
  ProjectMap,
  ProjectOverview,
  ProjectSummary,
  SearchResponse,
  Settings,
  StartupStatus,
} from "@/lib/bindings";

const NOW = Date.now();
const MIN = 60_000;
const HOUR = 60 * MIN;

const DATA_DIR = "C:\\Users\\voce\\AppData\\Local\\Nexus";

export const startup: StartupStatus = {
  ready: true,
  dataDir: DATA_DIR,
  error: null,
  schemaVersion: 7,
  appliedMigrations: [],
  backupCreated: false,
  recoveredOperations: 0,
};

export const appInfo: AppInfo = {
  appVersion: "0.2.0",
  coreVersion: "0.2.0",
  buildProfile: "release",
  os: "windows",
  arch: "x86_64",
  webviewVersion: "141.0.3537.71",
  tauriVersion: "2.12.3",
};

export const settings: Settings = {
  theme: "dark",
  reducedMotion: "system",
  sidebarCollapsed: false,
  indexingThreads: 0,
  maxFileSizeKb: 1024,
  watchProjects: true,
  updateCheck: "on",
  updateLastCheckMs: NOW - 2 * HOUR,
};

function project(
  id: number,
  name: string,
  indexed: number,
  lines: number,
  bytes: number,
  ago: number,
  commit: string | null,
): ProjectSummary {
  return {
    project: {
      id,
      name,
      rootPath: `C:\\Users\\voce\\code\\${name}`,
      createdAt: NOW - 30 * 24 * HOUR,
      lastOpenedAt: NOW - ago,
      lastIndexedAt: NOW - ago,
      indexGeneration: 12 + id,
      headRef: commit ? "refs/heads/main" : null,
      headCommit: commit,
    },
    rootExists: true,
    counts: {
      total: indexed + 14,
      indexed,
      skipped: 14,
      failed: 0,
      indexedBytes: bytes,
      indexedLines: lines,
    },
    activeJob: null,
    lastJob: null,
  };
}

const MAIN = project(
  1,
  "nexus",
  531,
  62_573,
  2_306_867,
  2 * MIN,
  "9def3af2c1b04e7a8d6c5f1e2a3b4c5d6e7f8091",
);

export const projects: ProjectSummary[] = [
  MAIN,
  project(
    2,
    "shopfront",
    1_284,
    148_210,
    5_801_204,
    35 * MIN,
    "4b1c9e07aa21f3d6b8e0c4a5d7f9b2e1c3a5d7f9",
  ),
  project(3, "otimiza", 412, 38_904, 1_420_118, 3 * HOUR, null),
];

export function overview(id: number): ProjectOverview {
  const summary = projects.find((p) => p.project.id === id) ?? MAIN;
  return {
    summary,
    languages: [
      { language: "rust", files: 182, bytes: 1_020_000, lines: 28_410 },
      { language: "typescript", files: 214, bytes: 860_000, lines: 21_388 },
      { language: "markdown", files: 61, bytes: 240_000, lines: 6_120 },
      { language: "go", files: 18, bytes: 44_000, lines: 1_204 },
      { language: "json", files: 40, bytes: 120_000, lines: 3_002 },
    ],
    kinds: [
      { kind: "code", files: 396 },
      { kind: "test", files: 74 },
      { kind: "doc", files: 61 },
    ],
    statuses: [{ status: "indexed", files: summary.counts.indexed }],
    topDirectories: [
      { name: "crates", files: 168 },
      { name: "apps", files: 241 },
      { name: "docs", files: 52 },
    ],
    manifests: ["Cargo.toml", "package.json", "go.mod"],
    ciFiles: [".github/workflows/ci.yml", ".github/workflows/release.yml"],
    mostChanged: [],
  };
}

export function jobs(projectId: number): IndexJob[] {
  return [
    {
      id: `job-${projectId}-1`,
      projectId,
      mode: "incremental",
      trigger: "watch",
      status: "succeeded",
      phase: "done",
      startedAt: NOW - 2 * MIN,
      finishedAt: NOW - 2 * MIN + 412,
      discovered: 545,
      toProcess: 3,
      processed: 3,
      unchanged: 528,
      skipped: 14,
      failed: 0,
      removed: 0,
      renamed: 0,
      generation: 13,
      errorMessage: null,
      opId: "op-1",
    } as unknown as IndexJob,
  ];
}

function op(kind: string, ago: number, durationMs: number, detail: unknown = {}): OperationRecord {
  return {
    id: `op-${kind}-${ago}`,
    kind,
    projectId: 1,
    status: "succeeded",
    startedAt: NOW - ago,
    heartbeatAt: NOW - ago,
    finishedAt: NOW - ago + durationMs,
    durationMs,
    detail: detail as OperationRecord["detail"],
    errorCode: null,
    errorMessage: null,
  };
}

export const operations: OperationRecord[] = [
  op("index.run", 2 * MIN, 412),
  op("mcp.tool", 6 * MIN, 37, { tool: "search_project", isError: false }),
  op("context.compile", 14 * MIN, 188),
  op("mcp.tool", 21 * MIN, 52, { tool: "build_context_pack", isError: false }),
  op("freshness.scan", 40 * MIN, 326),
  op("mcp.session", 55 * MIN, 1_200, { clientName: "claude-code" }),
  op("updates.check", 2 * HOUR, 640, { manual: false, result: "up_to_date" }),
];

export const diagnostics: DiagnosticsReport = {
  coreVersion: "0.2.0",
  sqliteVersion: "3.53.2",
  fts5Available: true,
  dataDir: DATA_DIR,
  database: { path: "nexus.db", exists: true, sizeBytes: 8_493_056 },
  wal: { path: "nexus.db-wal", exists: true, sizeBytes: 1_048_576 },
  logsDir: "logs",
  logsSizeBytes: 182_000,
  logFiles: 3,
  backupsDir: "backups",
  backups: [],
  schemaVersion: 7,
  latestSchemaVersion: 7,
  migrations: [1, 2, 3, 4, 5, 6, 7].map((version) => ({
    version,
    name:
      [
        "foundation",
        "projects_index",
        "context_packs",
        "memory_freshness",
        "analysis",
        "edges_targets",
        "shared_memory",
      ][version - 1] ?? "",
    checksum: "bbc25775ace5abcdef0123",
    appliedAt: NOW - (7 - version) * 24 * HOUR,
    durationMs: 4 + version,
  })),
  operationsTotal: 1_284,
  lastIntegrityCheck: null,
};

export const memoryCounts = {
  proposed: 2,
  confirmed: 9,
  rejected: 1,
  superseded: 1,
  archived: 0,
  stale: 0,
} as unknown as MemoryCounts;

function memory(
  id: number,
  kind: Memory["kind"],
  status: Memory["status"],
  title: string,
  body: string,
): Memory {
  return {
    id: `01a13000-0000-7000-8000-${String(id).padStart(12, "0")}`,
    projectId: 1,
    kind,
    status,
    title,
    body,
    origin: "user",
    originPath: null,
    originHash: null,
    supersedes: null,
    supersededBy: null,
    revision: 1,
    createdAt: NOW - id * HOUR,
    updatedAt: NOW - id * HOUR,
    confirmedAt: status === "confirmed" ? NOW - id * HOUR : null,
    refs: [{ relPath: "docs/architecture.md", startLine: 12, endLine: 40, contentHash: "abc" }],
  };
}

export const memories: Memory[] = [
  memory(
    1,
    "decision",
    "confirmed",
    "SQLite compartilhado entre app, CLI e MCP",
    "Um só banco local em WAL; leitores concorrentes e um escritor por vez.",
  ),
  memory(
    2,
    "convention",
    "confirmed",
    "Textos da interface em português do Brasil",
    "Identificadores e contratos continuam em inglês.",
  ),
  memory(
    3,
    "decision",
    "proposed",
    "Atualizações só com permissão",
    "O app pergunta na primeira abertura; sem resposta, não consulta a rede.",
  ),
  memory(
    4,
    "lesson",
    "confirmed",
    "Comparar caminhos pelo caminho real",
    "Nomes curtos do Windows (RUNNER~1) enganavam a comparação por texto.",
  ),
];

export const freshnessSummary = {
  projectId: 1,
  open: 3,
  warnings: 2,
  contradictions: 1,
  needsReview: 1,
  byKind: [],
  lastScan: { scannedAt: NOW - 40 * MIN, durationMs: 326, docsChecked: 61, memoriesChecked: 13 },
  scanOutdated: false,
} as unknown as FreshnessSummary;

export const issues: FreshnessIssue[] = [
  {
    id: 7,
    projectId: 1,
    kind: "doc_missing_path",
    severity: "warning",
    status: "open",
    needsReview: false,
    contradiction: false,
    title: "docs/setup.md cita um arquivo que não existe mais",
    reason: "`scripts/install.ps1` foi removido no commit 4b1c9e0.",
    recommendation: "Atualize o documento ou remova a referência.",
    sources: [{ relPath: "docs/setup.md", line: 18, cited: "scripts/install.ps1" }],
    suggestion: null,
    subjectPath: "docs/setup.md",
    memoryId: null,
    relatedMemoryId: null,
    firstDetectedAt: NOW - 3 * HOUR,
    lastDetectedAt: NOW - 40 * MIN,
    resolvedAt: null,
    dismissedAt: null,
  } as unknown as FreshnessIssue,
];

export const integration: IntegrationInfo = {
  cliPath: "C:\\Users\\voce\\AppData\\Local\\NEXUS\\nexus.exe",
  dataDir: DATA_DIR,
  serveArgs: ["--data-dir", DATA_DIR, "mcp", "serve"],
  claudeCommand: `claude mcp add nexus -- C:\\Users\\voce\\AppData\\Local\\NEXUS\\nexus.exe --data-dir ${DATA_DIR} mcp serve`,
  clients: [
    {
      id: "claude-code",
      name: "Claude Code",
      kind: "command",
      text: `claude mcp add nexus -- C:\\Users\\voce\\AppData\\Local\\NEXUS\\nexus.exe --data-dir ${DATA_DIR} mcp serve`,
      whereHint: "Rode uma vez num terminal.",
    },
    {
      id: "cursor",
      name: "Cursor",
      kind: "json",
      text: '{\n  "mcpServers": {\n    "nexus": { "type": "stdio", "command": "nexus.exe" }\n  }\n}',
      whereHint: "Cole em ~/.cursor/mcp.json.",
    },
  ],
  protocolVersions: ["2026-07-28", "2025-11-25"],
  tools: [
    {
      name: "search_project",
      title: "Search project",
      description: "Hybrid search with reasons.",
      readOnly: true,
    },
    {
      name: "build_context_pack",
      title: "Build Context Pack",
      description: "Compile a pack for a task.",
      readOnly: false,
    },
  ],
};

export const search: SearchResponse = {
  query: "onde a sessão expira",
  mode: "hybrid",
  analysis: { concepts: [], intents: [], mentionedPaths: [], ignored: [] },
  hits: [
    {
      relPath: "src/auth/session.ts",
      language: "typescript",
      kind: "code",
      startLine: 24,
      endLine: 31,
      snippet:
        "/** Sessões expiram em 30 minutos sem uso. */\nexport const SESSION_TTL_MS = 30 * 60_000;\n\nexport function isExpired(s: Session, now = Date.now()) {\n  return now - s.lastSeenAt > SESSION_TTL_MS;\n}",
      matchLines: [24, 25],
      score: 0.93,
      reasons: [
        { kind: "content", label: "Conteúdo corresponde à consulta", weight: 0.4 },
        { kind: "definition", label: "Declara `isExpired`", weight: 0.1 },
      ],
      freshness: "fresh",
      contentHash: "abc",
      indexedAt: NOW - 2 * MIN,
      alsoIn: [],
    },
  ],
  candidates: 12,
  tookMs: 37,
  indexGeneration: 13,
  warnings: [],
};

export const packs: ContextPackSummary[] = [];
export const pack: ContextPack | null = null;

export const analysis: AnalysisOverview = {
  graph: null,
  languages: [],
  notAnalyzed: [],
  mostImported: [
    { name: "crates/nexus-core/src/lib.rs", count: 41 },
    { name: "apps/desktop/src/lib/ipc.ts", count: 37 },
  ],
  externalPackages: [
    { name: "react", count: 64 },
    { name: "serde", count: 52 },
  ],
  coverage: [],
};

/** PageRank como no núcleo: amortecimento 0,85, 30 iterações, nós sem saída distribuem a nota. */
function pagerank(ids: string[], edges: [string, string][]): Map<string, number> {
  const n = ids.length;
  const out = new Map(ids.map((id) => [id, [] as string[]]));
  for (const [from, to] of edges) out.get(from)?.push(to);
  let rank = new Map(ids.map((id) => [id, 1 / Math.max(1, n)]));
  for (let it = 0; it < 30; it++) {
    const dangling = ids
      .filter((id) => (out.get(id) ?? []).length === 0)
      .reduce((sum, id) => sum + (rank.get(id) ?? 0), 0);
    const next = new Map(ids.map((id) => [id, (0.15 + 0.85 * dangling) / n]));
    for (const id of ids) {
      const links = out.get(id) ?? [];
      for (const to of links) {
        next.set(to, (next.get(to) ?? 0) + (0.85 * (rank.get(id) ?? 0)) / links.length);
      }
    }
    rank = next;
  }
  return rank;
}

/** Arquivo de exemplo (60 linhas) para a prévia; respeita o intervalo pedido. */
export function fileContext(relPath: string, start: number, end: number): FileContext {
  const python = relPath.endsWith(".py");
  const total = 60;
  const lines = Array.from({ length: total }, (_, i) =>
    i === 0
      ? `${python ? "#" : "//"} ${relPath}`
      : python
        ? (`def exemplo_${i}():\n    return "linha ${i + 1} da prévia"`.split("\n")[i % 2] ?? "")
        : `export const exemplo${i} = "linha ${i + 1} da prévia";`,
  );
  // Como no núcleo: intervalo invertido é erro, não é corrigido em silêncio.
  // Como no núcleo: o início vale no mínimo 1; um fim antes do início é erro.
  const from = Math.max(1, start);
  if (end < from) {
    throw new Error(`intervalo de linhas inválido: ${start}–${end}`);
  }
  const first = Math.min(from, total);
  const last = Math.min(end, total);
  return {
    relPath,
    language: python ? "python" : "typescript",
    kind: "code",
    startLine: first,
    endLine: last,
    totalLines: total,
    text: lines.slice(first - 1, last).join("\n"),
    freshness: "fresh",
    contentHash: "0".repeat(64),
    indexedAt: NOW,
  };
}

/** Grafo de exemplo para o Mapa: um app TS com serviços, rotas e um utilitário em Python. */
export function projectMap(request: MapRequest): ProjectMap {
  const folders = ["src/server/routes", "src/services", "src/db", "src/auth", "src/ui", "scripts"];
  const files: { id: string; language: string }[] = [];
  folders.forEach((folder, f) => {
    const count = [9, 8, 4, 5, 10, 3][f] ?? 3;
    for (let i = 0; i < count; i++) {
      const python = folder === "scripts";
      files.push({
        id: `${folder}/${python ? "job" : "mod"}${i}.${python ? "py" : "ts"}`,
        language: python ? "python" : "typescript",
      });
    }
  });
  const edges: { from: string; to: string }[] = [];
  const pick = (folder: string, i: number) =>
    files.filter((x) => x.id.startsWith(`${folder}/`))[i]?.id ?? "";
  for (let i = 0; i < 9; i++) {
    edges.push({ from: pick("src/server/routes", i), to: pick("src/services", i % 8) });
    edges.push({ from: pick("src/server/routes", i), to: pick("src/auth", 0) });
  }
  for (let i = 0; i < 8; i++) {
    edges.push({ from: pick("src/services", i), to: pick("src/db", i % 4) });
    if (i % 3 === 0) edges.push({ from: pick("src/services", i), to: pick("src/services", 0) });
  }
  for (let i = 1; i < 5; i++) edges.push({ from: pick("src/auth", i), to: pick("src/db", 0) });
  for (let i = 0; i < 10; i++) edges.push({ from: pick("src/ui", i), to: pick("src/ui", i % 3) });
  for (let i = 0; i < 3; i++) edges.push({ from: pick("scripts", i), to: pick("src/db", 1) });
  const real = edges.filter((e) => e.from && e.to && e.from !== e.to);
  const inside = (id: string) => !request.folder || id.startsWith(`${request.folder}/`);
  const key = (id: string) => (request.groupByFolder ? id.slice(0, id.lastIndexOf("/")) : id);
  const weights = new Map<string, number>();
  for (const e of real) {
    if (!inside(e.from) || !inside(e.to) || key(e.from) === key(e.to)) continue;
    const k = `${key(e.from)}\u0000${key(e.to)}`;
    weights.set(k, (weights.get(k) ?? 0) + 1);
  }
  // Como no núcleo: só entram pontas de imports com os dois lados dentro do filtro.
  const ids = [
    ...new Set(
      real
        .filter((e) => inside(e.from) && inside(e.to))
        .flatMap((e) => [e.from, e.to])
        .map(key),
    ),
  ];
  const inDeg = (id: string) => [...weights.keys()].filter((k) => k.endsWith(`\u0000${id}`)).length;
  const outDeg = (id: string) =>
    [...weights.keys()].filter((k) => k.startsWith(`${id}\u0000`)).length;
  const nodes: MapNode[] = ids.map((id) => ({
    id,
    folder: request.groupByFolder ? id : id.slice(0, id.lastIndexOf("/")),
    language: files.find((x) => key(x.id) === id)?.language ?? null,
    files: files.filter((x) => key(x.id) === id).length,
    inDegree: inDeg(id),
    outDegree: outDeg(id),
    centrality: 0,
  }));
  // Mesmo PageRank do núcleo (`analysis::map`), normalizado pelo maior; ordena por ele.
  const rank = pagerank(
    ids,
    [...weights.keys()].map((k) => k.split("\u0000") as [string, string]),
  );
  const top = Math.max(...rank.values(), Number.MIN_VALUE);
  for (const n of nodes) n.centrality = (rank.get(n.id) ?? 0) / top;
  nodes.sort(
    (a, b) => b.centrality - a.centrality || b.inDegree - a.inDegree || a.id.localeCompare(b.id),
  );
  return {
    nodes,
    edges: [...weights].map(([k, weight]) => {
      const [from = "", to = ""] = k.split("\u0000");
      return { from, to, weight };
    }),
    totalNodes: nodes.length,
    totalEdges: weights.size,
    grouped: request.groupByFolder,
  };
}

/** Prévia de "Exportar para o repositório". */
export const exportPreview: ExportPreview = {
  directory: ".nexus/memory",
  items: [
    {
      relPath: ".nexus/memory/sessoes-no-servidor-com-cookie-httponly-0a1b2c3d.md",
      memoryId: "0190f0aa-1111-7222-8333-00000a1b2c3d",
      title: "Sessões no servidor com cookie HttpOnly",
      action: "create",
    },
    {
      relPath: ".nexus/memory/valores-em-centavos-4e5f6a7b.md",
      memoryId: "0190f0aa-1111-7222-8333-00004e5f6a7b",
      title: "Valores em centavos (inteiros)",
      action: "update",
    },
    {
      relPath: ".nexus/memory/commits-em-portugues-8c9d0e1f.md",
      memoryId: "0190f0aa-1111-7222-8333-00008c9d0e1f",
      title: "Commits em português",
      action: "create",
    },
    {
      relPath: ".nexus/memory/testes-com-fixtures-reais-2a3b4c5d.md",
      memoryId: "0190f0aa-1111-7222-8333-00002a3b4c5d",
      title: "Testes com fixtures reais",
      action: "unchanged",
    },
  ],
  obsolete: [{ relPath: ".nexus/memory/jwt-no-cliente-99aa88bb.md", title: "JWT no cliente" }],
  token: "previa",
};
