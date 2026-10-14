import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import type { ContextPack, ProjectSummary, SearchResponse } from "@/lib/bindings";
import { renderApp, setupIpc } from "@/test/harness";

const NOW = Date.now();

const project: ProjectSummary = {
  project: {
    id: 1,
    name: "acme",
    rootPath: "C:\\repos\\acme",
    createdAt: NOW,
    lastOpenedAt: NOW,
    lastIndexedAt: NOW,
    indexGeneration: 2,
    headRef: null,
    headCommit: null,
  },
  rootExists: true,
  counts: { total: 10, indexed: 10, skipped: 0, failed: 0, indexedBytes: 1000, indexedLines: 100 },
  activeJob: null,
  lastJob: null,
};

const response: SearchResponse = {
  query: "validateSession",
  mode: "hybrid",
  analysis: {
    concepts: [{ term: "validatesession", variants: ["session", "validate"] }],
    intents: [],
    mentionedPaths: [],
    ignored: [],
  },
  hits: [
    {
      relPath: "src/auth/session.ts",
      language: "typescript",
      kind: "code",
      startLine: 24,
      endLine: 27,
      snippet:
        "/** Valida o cookie de sessão. */\nexport async function validateSession(id: string) {\n  return null;\n}",
      matchLines: [25],
      score: 0.91,
      reasons: [
        { kind: "content", label: "Conteúdo corresponde à consulta", weight: 0.4 },
        { kind: "definition", label: "Declara `validateSession`", weight: 0.1 },
        { kind: "file_kind", label: "Lockfile (raramente relevante)", weight: -0.3 },
      ],
      freshness: "modified",
      contentHash: "abc",
      indexedAt: NOW,
      alsoIn: ["legacy/session.ts"],
    },
  ],
  candidates: 4,
  tookMs: 6,
  indexGeneration: 2,
  warnings: ["Alguns arquivos mudaram desde a indexação; atualize o índice para trechos exatos."],
};

function pack(stale = 0): ContextPack {
  return {
    plan: {
      schemaVersion: 1,
      id: "0190aaaa-bbbb-7ccc-8ddd-eeeeffff0000",
      projectId: 1,
      projectName: "acme",
      task: "tempo de expiração da sessão",
      createdAt: NOW,
      indexGeneration: 2,
      headRef: null,
      headCommit: null,
      analysis: { terms: ["tempo"], expandedTerms: ["ttl"], intents: [], mentionedPaths: [] },
      summary: "2 fonte(s): 1 principal(is), 0 relacionada(s), 1 teste(s).",
      sources: [
        {
          relPath: "src/auth/session.ts",
          role: "primary",
          startLine: 1,
          endLine: 3,
          contentHash: "abc",
          score: 0.9,
          reasons: ["Declara `createSession`"],
          language: "typescript",
          kind: "code",
          estimatedTokens: 120,
        },
        {
          relPath: "tests/auth/session.test.ts",
          role: "test",
          startLine: 1,
          endLine: 2,
          contentHash: "def",
          score: 0,
          reasons: ["Teste com nome correspondente a src/auth/session.ts"],
          language: "typescript",
          kind: "test",
          estimatedTokens: 80,
        },
      ],
      omitted: [],
      risks: [],
      limitations: ["Recuperação por texto, caminho, nomes de arquivo e heurísticas."],
      tokenBudget: 8000,
      estimatedTokens: 200,
      memories: [],
    },
    excerpts: [
      { startLine: 1, endLine: 3, text: "a\nb\nc", freshness: stale > 0 ? "modified" : "fresh" },
      { startLine: 1, endLine: 2, text: "t1\nt2", freshness: "fresh" },
    ],
    staleSources: stale,
    memories: [],
    issues: [],
    recentChanges: [],
  };
}

async function open(user: ReturnType<typeof userEvent.setup>, name: string) {
  const nav = await screen.findByRole("navigation", { name: "Navegação principal" });
  await user.click(within(nav).getByRole("button", { name }));
}

describe("Busca", () => {
  it("mostra resultados com motivos, atualidade, trecho e prévia", async () => {
    const user = userEvent.setup();
    const { calls } = setupIpc({
      list_projects: () => [project],
      search_project: () => response,
      get_file_context: () => ({
        relPath: "src/auth/session.ts",
        language: "typescript",
        kind: "code",
        startLine: 4,
        endLine: 6,
        totalLines: 40,
        text: "linha 4\nlinha 5\nlinha 6",
        freshness: "modified",
        contentHash: "abc",
        indexedAt: NOW,
      }),
    });
    renderApp();
    await open(user, "Busca");
    await user.type(await screen.findByRole("searchbox"), "validateSession");
    await user.click(screen.getByRole("button", { name: "Buscar" }));

    const results = await screen.findByRole("list", { name: "Resultados" });
    expect(within(results).getByText("src/auth/session.ts")).toBeInTheDocument();
    expect(within(results).getByText("Declara `validateSession`")).toBeInTheDocument();
    // Motivos negativos não aparecem como "motivo de relevância".
    expect(within(results).queryByText(/Lockfile/)).toBeNull();
    expect(within(results).getAllByText("Modificado").length).toBeGreaterThan(0);
    expect(within(results).getByText("legacy/session.ts")).toBeInTheDocument();
    const matched = results.querySelector('[data-line="25"]');
    expect(matched).toHaveAttribute("data-match", "true");
    expect(screen.getByRole("note")).toHaveTextContent("mudaram desde a indexação");
    expect(screen.getByText(/validatesession → session, validate/)).toBeInTheDocument();
    expect(calls).toContainEqual({
      cmd: "search_project",
      args: { request: { projectId: 1, query: "validateSession", limit: 20 } },
    });

    await user.click(within(results).getByRole("button", { name: /src\/auth\/session\.ts/ }));
    const preview = await screen.findByRole("complementary", { name: "Prévia" });
    expect(await within(preview).findByText("linha 5")).toBeInTheDocument();
  });

  it("filtra por tipo e leva a consulta para um Context Pack", async () => {
    const user = userEvent.setup();
    const { calls } = setupIpc({
      list_projects: () => [project],
      search_project: () => response,
      list_context_packs: () => [],
    });
    renderApp();
    await open(user, "Busca");
    await user.click(await screen.findByRole("button", { name: "Testes" }));
    await user.type(screen.getByRole("searchbox"), "sessão");
    await user.keyboard("{Enter}");
    await screen.findByRole("list", { name: "Resultados" });
    expect(calls).toContainEqual({
      cmd: "search_project",
      args: { request: { projectId: 1, query: "sessão", limit: 20, kinds: ["test"] } },
    });
    await user.click(screen.getByRole("button", { name: "Criar Context Pack com esta busca" }));
    expect(await screen.findByRole("textbox", { name: "Descreva a tarefa" })).toHaveValue(
      "validateSession",
    );
  });
});

describe("Context Packs", () => {
  it("compila, mostra seções e exporta respeitando exclusões e notas", async () => {
    const user = userEvent.setup();
    const { calls } = setupIpc({
      list_projects: () => [project],
      list_context_packs: () => [],
      compile_context_pack: () => pack(),
      get_context_pack: () => pack(),
      render_context_pack: () => "# Context Pack",
      save_context_pack: () => "C:\\saida\\pack.md",
    });
    renderApp();
    await open(user, "Context Packs");
    await user.type(
      await screen.findByRole("textbox", { name: "Descreva a tarefa" }),
      "tempo de expiração da sessão",
    );
    await user.click(screen.getByRole("button", { name: "Compilar Context Pack" }));

    expect(
      await screen.findByRole("heading", { name: "tempo de expiração da sessão" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Arquivos principais" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Testes relacionados" })).toBeInTheDocument();
    expect(screen.getByText(/~200 de 8000 tokens estimados/)).toBeInTheDocument();

    await user.click(
      screen.getByRole("checkbox", { name: "Incluir no export: tests/auth/session.test.ts" }),
    );
    expect(screen.getByText(/~120 de 8000 tokens estimados/)).toBeInTheDocument();
    await user.type(screen.getByRole("textbox", { name: /Notas/ }), "manter API");
    await user.click(screen.getByRole("button", { name: "Copiar Markdown" }));
    await waitFor(() =>
      expect(calls).toContainEqual({
        cmd: "render_context_pack",
        args: {
          packId: "0190aaaa-bbbb-7ccc-8ddd-eeeeffff0000",
          format: "markdown",
          options: {
            excludedPaths: ["tests/auth/session.test.ts"],
            excludedMemories: [],
            notes: "manter API",
          },
        },
      }),
    );
    await user.click(screen.getByRole("button", { name: "Salvar .json" }));
    expect(await screen.findByText("Salvo em C:\\saida\\pack.md")).toBeInTheDocument();
  });

  it("avisa quando fontes mudaram desde a criação do pack", async () => {
    const user = userEvent.setup();
    setupIpc({
      list_projects: () => [project],
      list_context_packs: () => [
        {
          id: "0190aaaa-bbbb-7ccc-8ddd-eeeeffff0000",
          projectId: 1,
          task: "tempo de expiração da sessão",
          createdAt: NOW,
          indexGeneration: 2,
          fileCount: 2,
          estimatedTokens: 200,
          tokenBudget: 8000,
        },
      ],
      get_context_pack: () => pack(1),
    });
    renderApp();
    await open(user, "Context Packs");
    await user.click(await screen.findByRole("button", { name: /tempo de expiração da sessão/ }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "1 fonte mudou desde a criação deste pack.",
    );
  });
});

describe("Dependências na prévia", () => {
  it("mostra declarações, imports e quem importa, e navega pelo grafo", async () => {
    const user = userEvent.setup();
    const { calls } = setupIpc({
      list_projects: () => [project],
      search_project: () => response,
      get_file_context: (args) => ({
        relPath: args.relPath as string,
        language: "typescript",
        kind: "code",
        startLine: 1,
        endLine: 3,
        totalLines: 40,
        text: `conteúdo de ${args.relPath as string}`,
        freshness: "fresh",
        contentHash: "abc",
        indexedAt: NOW,
      }),
      get_file_analysis: (args) => ({
        relPath: args.relPath as string,
        language: "typescript",
        analyzed: true,
        symbols: [
          {
            name: "validateSession",
            kind: "function",
            line: 25,
            endLine: 34,
            exported: true,
            container: null,
          },
        ],
        imports: [
          {
            fromPath: "src/auth/session.ts",
            line: 4,
            specifier: "../db/client",
            kind: "import",
            toPath: "src/db/client.ts",
            external: null,
          },
          {
            fromPath: "src/auth/session.ts",
            line: 1,
            specifier: "node:crypto",
            kind: "import",
            toPath: null,
            external: "node:crypto",
          },
        ],
        importedBy: [
          {
            fromPath: "src/server/routes/auth.routes.ts",
            line: 3,
            specifier: "../../auth/session",
            kind: "import",
            toPath: "src/auth/session.ts",
            external: null,
          },
        ],
      }),
    });
    renderApp();
    await open(user, "Busca");
    await user.type(await screen.findByRole("searchbox"), "validateSession");
    await user.click(screen.getByRole("button", { name: "Buscar" }));
    const results = await screen.findByRole("list", { name: "Resultados" });
    await user.click(within(results).getByRole("button", { name: /src\/auth\/session\.ts/ }));
    const preview = await screen.findByRole("complementary", { name: "Prévia" });
    const deps = await within(preview).findByRole("region", { name: "Dependências do arquivo" });
    expect(within(deps).getByText("validateSession")).toBeInTheDocument();
    expect(within(deps).getByText("função · 25")).toBeInTheDocument();
    expect(within(deps).getByText("pacote node:crypto")).toBeInTheDocument();
    await user.click(
      within(deps).getByRole("button", { name: "src/server/routes/auth.routes.ts" }),
    );
    expect(
      await within(preview).findByText("conteúdo de src/server/routes/auth.routes.ts"),
    ).toBeInTheDocument();
    expect(calls).toContainEqual({
      cmd: "get_file_context",
      args: {
        projectId: 1,
        relPath: "src/server/routes/auth.routes.ts",
        startLine: 1,
        endLine: 400,
      },
    });
    await user.click(within(preview).getByRole("button", { name: "Voltar ao resultado" }));
    expect(await within(preview).findByText("conteúdo de src/auth/session.ts")).toBeInTheDocument();
  });
});

describe("Histórico na prévia", () => {
  const fileContext = (args: Record<string, unknown>) => ({
    relPath: args.relPath as string,
    language: "typescript",
    kind: "code",
    startLine: 1,
    endLine: 3,
    totalLines: 40,
    text: "conteúdo",
    freshness: "fresh",
    contentHash: "abc",
    indexedAt: NOW,
  });

  async function openPreview() {
    const user = userEvent.setup();
    renderApp();
    await open(user, "Busca");
    await user.type(await screen.findByRole("searchbox"), "validateSession");
    await user.click(screen.getByRole("button", { name: "Buscar" }));
    const results = await screen.findByRole("list", { name: "Resultados" });
    await user.click(within(results).getByRole("button", { name: /src\/auth\/session\.ts/ }));
    const preview = await screen.findByRole("complementary", { name: "Prévia" });
    return within(preview).findByRole("region", { name: "Histórico do arquivo" });
  }

  it("lista os últimos commits do arquivo", async () => {
    const { calls } = setupIpc({
      list_projects: () => [project],
      search_project: () => response,
      get_file_context: fileContext,
      get_file_history: () => ({
        gitAvailable: true,
        commits: [
          {
            hash: "a".repeat(40),
            shortHash: "aaaaaaa",
            author: "Bruno",
            dateMs: NOW - 2 * 86_400_000,
            subject: "fix: sessão expirada",
          },
        ],
      }),
    });
    const history = await openPreview();
    expect(within(history).getByText("Mudanças recentes (1)")).toBeInTheDocument();
    expect(within(history).getByText("fix: sessão expirada")).toBeInTheDocument();
    expect(within(history).getByText(/Bruno · /)).toBeInTheDocument();
    expect(calls).toContainEqual({
      cmd: "get_file_history",
      args: { projectId: 1, relPath: "src/auth/session.ts" },
    });
  });

  it("explica quando o git não está instalado", async () => {
    setupIpc({
      list_projects: () => [project],
      search_project: () => response,
      get_file_context: fileContext,
      get_file_history: () => ({ gitAvailable: false, commits: [] }),
    });
    const history = await openPreview();
    expect(within(history).getByText(/Instale o git/)).toBeInTheDocument();
  });
});
