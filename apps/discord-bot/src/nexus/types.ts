/** Formas (parciais) das saídas das ferramentas MCP da NEXUS usadas pelo bot. */

export interface SearchHit {
  relPath: string;
  startLine: number;
  endLine: number;
  snippet: string;
  reasons: string[];
  /** `fresh` | `modified` | `missing` | `blocked` */
  freshness: string;
  language: string | null;
  score: number;
}

export interface SearchResult {
  hits: SearchHit[];
  warnings: string[];
  project: { id: number; name: string };
}

/** Item de `list_projects`. */
export interface ProjectStatus {
  id: number;
  name: string;
  rootPath: string;
  rootExists: boolean;
  indexedFiles: number;
  skippedFiles: number;
  indexGeneration: number;
  indexing: boolean;
  lastIndexedAt: number | null;
  headCommit: string | null;
}

export interface PackSource {
  relPath: string;
  startLine: number;
  endLine: number;
}

/** Saída de `build_context_pack` com `format: "markdown"`. */
export interface PackResult {
  packId: string;
  summary: string;
  estimatedTokens: number;
  tokenBudget: number;
  export: string;
  sources: PackSource[];
}
