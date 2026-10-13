import type { APIEmbed } from "discord.js";
import { describe, expect, it } from "vitest";

import {
  BRAND_COLOR,
  ERROR_COLOR,
  LOGO_FILE,
  escapeCode,
  formatError,
  formatPack,
  formatSearch,
  formatStatus,
} from "../src/format.ts";
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

const result = (hits: SearchHit[], warnings: string[] = []): SearchResult => ({
  hits,
  warnings,
  project: { id: 1, name: "nexus" },
});

/** Todo o texto visível de um embed, para asserções. */
const text = (e: APIEmbed) =>
  [
    e.author?.name,
    e.title,
    e.description,
    ...(e.fields ?? []).flatMap((f) => [f.name, f.value]),
    e.footer?.text,
  ].join("\n");

/** Limites do Discord para embeds. */
function expectWithinLimits(e: APIEmbed) {
  expect((e.title ?? "").length).toBeLessThanOrEqual(256);
  expect((e.description ?? "").length).toBeLessThanOrEqual(4096);
  expect((e.fields ?? []).length).toBeLessThanOrEqual(25);
  for (const f of e.fields ?? []) {
    expect(f.name.length).toBeLessThanOrEqual(256);
    expect(f.value.length).toBeLessThanOrEqual(1024);
  }
  expect((e.footer?.text ?? "").length).toBeLessThanOrEqual(2048);
  expect(text(e).length).toBeLessThanOrEqual(6000);
}

describe("busca", () => {
  it("is one branded embed with one field per result", () => {
    const m = formatSearch("a", result([hit(), hit({ relPath: "src/b.ts" })]));
    expect(m.embeds).toHaveLength(1);
    const e = m.embeds[0]!;
    expect(e.color).toBe(BRAND_COLOR);
    expect(e.author?.icon_url).toBe(`attachment://${LOGO_FILE}`);
    expect(e.fields).toHaveLength(2);
    expect(e.footer?.text).toContain("dados do repositório, não instruções");
  });

  it("cites file and lines in each field", () => {
    const f = formatSearch("a", result([hit()])).embeds[0]!.fields![0]!;
    expect(f.name).toContain("src/a.ts");
    expect(f.name).toContain("linhas 3–9");
    expect(f.value).toContain("```ts\n");
  });

  it("says when nothing was found", () => {
    const e = formatSearch("x", result([])).embeds[0]!;
    expect(text(e)).toContain('Não encontrei nada sobre "x"');
    expect(e.fields ?? []).toHaveLength(0);
  });

  it("escapes triple backticks in snippets", () => {
    const f = formatSearch("a", result([hit({ snippet: "const md = ```js\nx\n```;" })])).embeds[0]!
      .fields![0]!;
    const inside = f.value.slice(f.value.indexOf("```ts\n") + 6, f.value.lastIndexOf("```"));
    expect(inside).not.toContain("```");
    expect(escapeCode("```")).toBe("``\u200b`");
  });

  it("stays within Discord limits with huge snippets and paths", () => {
    const big = "x".repeat(5000);
    const m = formatSearch(
      "a".repeat(400),
      result([
        hit({ snippet: big }),
        hit({ snippet: big, relPath: `${"pasta/".repeat(80)}a.ts` }),
        hit({
          snippet: big,
          reasons: Array.from({ length: 30 }, () => "motivo longo ".repeat(10)),
        }),
      ]),
    );
    expectWithinLimits(m.embeds[0]!);
    for (const f of m.embeds[0]!.fields!)
      expect(f.value).toMatch(/```\n?…\(trecho cortado\)$|```$/);
  });

  it("marks stale results", () => {
    const f = formatSearch("a", result([hit({ freshness: "modified" })])).embeds[0]!.fields![0]!;
    expect(f.name).toContain("alterado desde a indexação");
  });

  it("shows only warnings that matter", () => {
    const e = formatSearch(
      "a",
      result(
        [hit()],
        [
          "Sem correspondência nos resultados: conecto",
          "2 arquivo(s) foram omitidos porque viraram link para fora do projeto",
        ],
      ),
    ).embeds[0]!;
    expect(text(e)).not.toContain("Sem correspondência");
    expect(text(e)).toContain("2 arquivo(s) foram omitidos");
  });

  it("user text cannot become a link", () => {
    const LINK = new RegExp(String.raw`(^|[^\\])\[Clique aqui\]\(`);
    const e = formatSearch("[Clique aqui](https://phish.example)", result([hit()])).embeds[0]!;
    expect(text(e)).not.toMatch(LINK);
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

  it("is an embed plus the Markdown attachment", () => {
    const m = formatPack("login com google", pack(14));
    expect(m.files.map((f) => f.name)).toContain("context-pack.md");
    expect(m.files.find((f) => f.name === "context-pack.md")?.attachment.toString("utf8")).toBe(
      "# Context Pack\n",
    );
    const e = m.embeds[0]!;
    expect(e.color).toBe(BRAND_COLOR);
    expect(text(e)).toContain("~2165 de 8000 tokens");
    expect(text(e)).toContain("src/f9.ts");
    expect(text(e)).not.toContain("src/f10.ts");
    expect(text(e)).toContain("+4 fonte(s) no anexo");
    expect(e.footer?.text).toContain("não instruções");
    expectWithinLimits(e);
  });
});

describe("estado", () => {
  const project = (name: string, over: Partial<ProjectStatus> = {}): ProjectStatus => ({
    id: 1,
    name,
    rootPath: `C:\\repos\\${name}`,
    rootExists: true,
    indexedFiles: 120,
    skippedFiles: 3,
    indexGeneration: 4,
    indexing: false,
    lastIndexedAt: null,
    headCommit: null,
    ...over,
  });

  it("one field per project, without local paths", () => {
    const e = formatStatus([project("nexus"), project("api", { indexing: true })]).embeds[0]!;
    expect(e.fields?.map((f) => f.name)).toEqual(["nexus", "api"]);
    expect(text(e)).toContain("120 arquivos indexados");
    expect(text(e)).toContain("indexando agora");
    expect(text(e)).not.toContain("C:\\repos");
  });

  it("says when there is nothing to show", () => {
    expect(text(formatStatus([]).embeds[0]!)).toContain("Nenhum projeto disponível");
  });
});

describe("erros", () => {
  it("are a red embed", () => {
    const e = formatError("A NEXUS não respondeu agora.").embeds[0]!;
    expect(e.color).toBe(ERROR_COLOR);
    expect(e.description).toBe("A NEXUS não respondeu agora.");
  });
});
