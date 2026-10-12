import { describe, expect, it } from "vitest";

import { MAX_MESSAGE, escapeCode, formatPack, formatSearch, formatStatus } from "../src/format.ts";
import type { PackResult, ProjectStatus, SearchHit, SearchResult } from "../src/nexus/types.ts";

const hit = (over: Partial<SearchHit> = {}): SearchHit => ({
  relPath: "src/a.ts",
  startLine: 3,
  endLine: 9,
  snippet: "export const a = 1;",
  reasons: ["Conteúdo corresponde à consulta"],
  freshness: "fresh",
  language: "typescript",
  score: 0.8,
  ...over,
});

const result = (hits: SearchHit[]): SearchResult => ({
  hits,
  warnings: [],
  project: { id: 1, name: "nexus" },
});

describe("busca", () => {
  it("cites file and lines", () => {
    const out = formatSearch("a", result([hit()]));
    expect(out).toContain("`src/a.ts:3-9`");
    expect(out).toContain("```ts\n");
  });

  it("says when nothing was found", () => {
    expect(formatSearch("x", result([]))).toContain('Não encontrei nada sobre "x"');
  });

  it("escapes triple backticks in snippets", () => {
    const out = formatSearch("a", result([hit({ snippet: "const md = ```js\nx\n```;" })]));
    const inside = out.slice(out.indexOf("```ts\n") + 6, out.lastIndexOf("```"));
    expect(inside).not.toContain("```");
    expect(escapeCode("```")).toBe("``\u200b`");
  });

  it("never exceeds 2000 chars", () => {
    const big = "x".repeat(5000);
    const out = formatSearch(
      "a",
      result([hit({ snippet: big }), hit({ snippet: big }), hit({ snippet: big })]),
    );
    expect(out.length).toBeLessThanOrEqual(MAX_MESSAGE);
    expect(out.trimEnd().endsWith("…(trecho cortado)")).toBe(true);
  });

  it("marks stale results", () => {
    expect(formatSearch("a", result([hit({ freshness: "modified" })]))).toContain(
      "(alterado desde a indexação)",
    );
  });
});

describe("Context Pack", () => {
  const pack = (n: number): PackResult => ({
    packId: "p1",
    summary: "12 fonte(s)",
    estimatedTokens: 2165,
    tokenBudget: 8000,
    export: "# Context Pack\n",
    sources: Array.from({ length: n }, (_, i) => ({
      relPath: `src/f${i}.ts`,
      startLine: 1,
      endLine: 10,
    })),
  });

  it("pack goes as attachment", () => {
    const out = formatPack("login com google", pack(14));
    expect(out.file.name).toBe("context-pack.md");
    expect(out.file.data.toString("utf8")).toBe("# Context Pack\n");
    expect(out.content).toContain("~2165 de 8000 tokens");
    expect(out.content).toContain("`src/f9.ts:1-10`");
    expect(out.content).not.toContain("src/f10.ts");
    expect(out.content).toContain("+4 fonte(s) no anexo");
    expect(out.content.length).toBeLessThanOrEqual(MAX_MESSAGE);
  });
});

describe("estado", () => {
  it("lists projects with their index", () => {
    const ps: ProjectStatus[] = [
      {
        id: 1,
        name: "nexus",
        rootPath: "C:\\repos\\nexus",
        rootExists: true,
        indexedFiles: 120,
        skippedFiles: 3,
        indexGeneration: 4,
        indexing: false,
        lastIndexedAt: null,
        headCommit: null,
      },
    ];
    const out = formatStatus(ps);
    expect(out).toContain("**nexus**");
    expect(out).toContain("120 arquivos indexados");
    expect(out).not.toContain("C:\\repos");
  });
});
