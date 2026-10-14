import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { parseRefs } from "@/features/memory/MemoryEditor";
import type {
  ContextPack,
  FreshnessIssue,
  FreshnessSummary,
  Memory,
  MemoryDetail,
  ProjectSummary,
} from "@/lib/bindings";
import { IpcError, renderApp, setupIpc } from "@/test/harness";

const NOW = Date.now();

const project: ProjectSummary = {
  project: {
    id: 1,
    name: "acme",
    rootPath: "C:\\repos\\acme",
    createdAt: NOW,
    lastOpenedAt: NOW,
    lastIndexedAt: NOW,
    indexGeneration: 3,
    headRef: null,
    headCommit: null,
  },
  rootExists: true,
  counts: { total: 10, indexed: 10, skipped: 0, failed: 0, indexedBytes: 1000, indexedLines: 100 },
  activeJob: null,
  lastJob: null,
};

const CONFIRMED_ID = "01a13000-0000-7000-8000-00000000c0f1";
const PROPOSAL_ID = "01a13000-0000-7000-8000-0000000a0b0c";

function memory(over: Partial<Memory> = {}): Memory {
  return {
    id: PROPOSAL_ID,
    projectId: 1,
    kind: "decision",
    status: "proposed",
    title: "ADR 0002 — Dinheiro em centavos inteiros",
    body: "Todo valor monetário usa Decimal com 4 casas.",
    origin: "adr_import",
    originPath: "docs/decisions/0002-money-in-cents.md",
    originHash: "abc",
    supersedes: CONFIRMED_ID,
    supersededBy: null,
    revision: 1,
    createdAt: NOW - 60_000,
    updatedAt: NOW - 60_000,
    confirmedAt: null,
    refs: [
      {
        relPath: "docs/decisions/0002-money-in-cents.md",
        startLine: null,
        endLine: null,
        contentHash: "abc",
      },
    ],
    ...over,
  };
}

function detail(m: Memory): MemoryDetail {
  return {
    memory: m,
    revisions: [
      {
        revision: 1,
        kind: m.kind,
        status: "proposed",
        title: m.title,
        body: m.body,
        refs: m.refs,
        actor: "adr_import",
        change: "created",
        note: "Importado de docs/decisions/0002-money-in-cents.md",
        changedAt: NOW - 60_000,
      },
    ],
    refChecks: m.refs.map((r) => ({ reference: r, state: "changed", movedTo: null })),
    supersedesTitle: "Dinheiro em centavos (versão confirmada)",
    supersededByTitle: null,
  };
}

const issue: FreshnessIssue = {
  id: 7,
  projectId: 1,
  kind: "memory_source_changed",
  severity: "warning",
  status: "open",
  needsReview: true,
  contradiction: true,
  title: "docs/decisions/0002-money-in-cents.md diz outra coisa agora",
  reason: "O texto da decisão mudou depois da importação.",
  recommendation: "Revise a proposta de atualização.",
  sources: [{ relPath: "docs/decisions/0002-money-in-cents.md", line: 4, cited: null }],
  suggestion: null,
  subjectPath: "docs/decisions/0002-money-in-cents.md",
  memoryId: CONFIRMED_ID,
  relatedMemoryId: PROPOSAL_ID,
  firstDetectedAt: NOW - 3_600_000,
  lastDetectedAt: NOW - 60_000,
  resolvedAt: null,
  dismissedAt: null,
  dismissNote: null,
};

const summary: FreshnessSummary = {
  projectId: 1,
  lastScan: {
    scannedAt: NOW - 60_000,
    indexGeneration: 3,
    durationMs: 46,
    docsChecked: 12,
    memoriesChecked: 4,
    openIssues: 1,
  },
  scanOutdated: false,
  open: 1,
  warnings: 1,
  needsReview: 1,
  contradictions: 1,
  dismissed: 0,
  byKind: [{ kind: "memory_source_changed", count: 1 }],
};

const counts = { proposed: 1, confirmed: 3, archived: 0, superseded: 0 };

function baseIpc(over: Parameters<typeof setupIpc>[0] = {}) {
  return setupIpc({
    list_projects: () => [project],
    get_memory_counts: () => counts,
    list_memories: () => [memory()],
    get_memory: () => detail(memory()),
    list_freshness_issues: () => [issue],
    get_freshness_summary: () => summary,
    ...over,
  });
}

async function open(user: ReturnType<typeof userEvent.setup>, name: string) {
  const nav = await screen.findByRole("navigation", { name: "Navegação principal" });
  await user.click(within(nav).getByRole("button", { name }));
}

describe("parseRefs", () => {
  it("aceita caminho, linha e intervalo e recusa intervalos invertidos", () => {
    expect(parseRefs("src/a.ts\n\n src/b.ts:7 \nsrc/c.ts:3-9")).toEqual({
      refs: [
        { relPath: "src/a.ts" },
        { relPath: "src/b.ts", startLine: 7, endLine: 7 },
        { relPath: "src/c.ts", startLine: 3, endLine: 9 },
      ],
      invalid: null,
    });
    expect(parseRefs("src/c.ts:9-3").invalid).toBe("src/c.ts:9-3");
  });
});

describe("Memória", () => {
  it("lista propostas com contagens e confirma com a revisão vista", async () => {
    const user = userEvent.setup();
    const { calls } = baseIpc({
      transition_memory: () => memory({ status: "confirmed", revision: 2 }),
    });
    renderApp();
    await open(user, "Memória");
    expect(await screen.findByRole("tab", { name: "Propostas (1)" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("tab", { name: "Confirmadas (3)" })).toBeInTheDocument();
    await user.click(await screen.findByRole("button", { name: /Dinheiro em centavos inteiros/ }));
    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: /ADR 0002 — Dinheiro em centavos inteiros/,
      }),
    ).toBeInTheDocument();
    // Proposta que substituiria uma decisão confirmada: aviso explícito.
    expect(screen.getByRole("note")).toHaveTextContent(
      "substituiria a decisão confirmada “Dinheiro em centavos (versão confirmada)”",
    );
    expect(screen.getByText("Mudou")).toBeInTheDocument();
    expect(screen.getByText(/Importação de ADR/)).toBeInTheDocument();
    // Alerta ligado à memória aparece no detalhe.
    expect(screen.getByText(issue.title)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Confirmar" }));
    await waitFor(() =>
      expect(calls).toContainEqual({
        cmd: "transition_memory",
        args: {
          memoryId: PROPOSAL_ID,
          transition: "confirm",
          expectedRevision: 1,
          note: null,
        },
      }),
    );
  });

  it("cria memória pelo editor, validando referências antes de enviar", async () => {
    const user = userEvent.setup();
    const { calls } = baseIpc({
      list_memories: () => [],
      create_memory: () =>
        memory({ id: CONFIRMED_ID, status: "confirmed", origin: "user", supersedes: null }),
    });
    renderApp();
    await open(user, "Memória");
    await user.click(await screen.findByRole("button", { name: "Nova memória" }));
    const dialog = await screen.findByRole("dialog");
    await user.selectOptions(within(dialog).getByLabelText("Tipo"), "convention");
    await user.type(within(dialog).getByLabelText("Título"), "Exportações nomeadas");
    await user.type(within(dialog).getByLabelText("Texto"), "Nada de export default.");
    const refs = within(dialog).getByLabelText(/Referências/);
    await user.type(refs, "src/a.ts:9-3");
    await user.click(within(dialog).getByRole("button", { name: "Salvar" }));
    expect(await within(dialog).findByRole("alert")).toHaveTextContent(
      "Referência inválida: src/a.ts:9-3",
    );
    expect(calls.some((c) => c.cmd === "create_memory")).toBe(false);

    await user.clear(refs);
    await user.type(refs, "src/a.ts:3-9");
    await user.click(within(dialog).getByLabelText(/Confirmar já/));
    await user.click(within(dialog).getByRole("button", { name: "Salvar" }));
    await waitFor(() =>
      expect(calls).toContainEqual({
        cmd: "create_memory",
        args: {
          request: {
            projectId: 1,
            kind: "convention",
            title: "Exportações nomeadas",
            body: "Nada de export default.",
            refs: [{ relPath: "src/a.ts", startLine: 3, endLine: 9 }],
            confirm: true,
          },
        },
      }),
    );
  });

  it("importa ADRs e mostra o resultado", async () => {
    const user = userEvent.setup();
    baseIpc({
      import_adrs: () => ({
        scanned: 3,
        created: 2,
        updated: 0,
        superseding: 1,
        unchanged: 0,
        skippedInactive: 0,
        skippedStale: ["docs/decisions/0004.md"],
        memoryIds: [],
      }),
    });
    renderApp();
    await open(user, "Memória");
    await user.click(await screen.findByRole("button", { name: "Importar ADRs" }));
    expect(await screen.findByRole("status")).toHaveTextContent(
      "2 proposta(s) nova(s), 0 atualizada(s), 1 de substituição, 0 sem mudança.",
    );
    expect(screen.getByText(/docs\/decisions\/0004.md/)).toBeInTheDocument();
  });

  it("mostra o aviso de conflito quando a memória mudou em outro lugar", async () => {
    const user = userEvent.setup();
    baseIpc({
      update_memory: () => {
        throw new IpcError({
          code: "conflict",
          message: "a memória foi alterada em outro lugar",
          opId: "op-1",
        });
      },
    });
    renderApp();
    await open(user, "Memória");
    await user.click(await screen.findByRole("button", { name: /Dinheiro em centavos inteiros/ }));
    await user.click(await screen.findByRole("button", { name: "Editar" }));
    const dialog = await screen.findByRole("dialog");
    await user.type(within(dialog).getByLabelText("Título"), " (v2)");
    await user.click(within(dialog).getByRole("button", { name: "Salvar" }));
    expect(await within(dialog).findByRole("alert")).toHaveTextContent(
      "A memória foi alterada em outro lugar",
    );
  });
});

describe("Atualidade", () => {
  it("mostra resumo, alerta com fontes e dispensa com motivo", async () => {
    const user = userEvent.setup();
    const { calls } = baseIpc({
      set_issue_dismissed: () => ({ ...issue, status: "dismissed", dismissedAt: NOW }),
    });
    renderApp();
    await open(user, "Atualidade");
    expect(
      await screen.findByRole("heading", { name: "Atualidade", level: 1 }),
    ).toBeInTheDocument();
    expect(await screen.findByText(/12 documento\(s\) e 4 memória\(s\)/)).toBeInTheDocument();
    const card = await screen.findByRole("article", { name: issue.title });
    expect(within(card).getByText("Contradição")).toBeInTheDocument();
    expect(within(card).getByText("Requer revisão humana")).toBeInTheDocument();
    expect(within(card).getByText("docs/decisions/0002-money-in-cents.md:4")).toBeInTheDocument();
    expect(within(card).getByText(/Revise a proposta de atualização/)).toBeInTheDocument();

    await user.click(within(card).getByRole("button", { name: "Dispensar" }));
    await user.type(within(card).getByLabelText("Motivo (opcional)"), "ADR antigo");
    await user.click(within(card).getByRole("button", { name: "Dispensar" }));
    await waitFor(() =>
      expect(calls).toContainEqual({
        cmd: "set_issue_dismissed",
        args: { issueId: 7, dismissed: true, note: "ADR antigo" },
      }),
    );
  });

  it("verifica sob demanda e abre a memória do alerta", async () => {
    const user = userEvent.setup();
    const { calls } = baseIpc({
      scan_freshness: () => ({
        projectId: 1,
        info: summary.lastScan,
        detected: 1,
        newIssues: 0,
        resolved: 0,
        byKind: [],
      }),
    });
    renderApp();
    await open(user, "Atualidade");
    await user.click(await screen.findByRole("button", { name: "Verificar agora" }));
    await waitFor(() =>
      expect(calls).toContainEqual({ cmd: "scan_freshness", args: { projectId: 1 } }),
    );
    await user.click(await screen.findByRole("button", { name: "Abrir a outra memória" }));
    expect(
      await screen.findByRole("heading", { level: 2, name: /Dinheiro em centavos inteiros/ }),
    ).toBeInTheDocument();
    expect(calls).toContainEqual({ cmd: "get_memory", args: { memoryId: PROPOSAL_ID } });
  });
});

describe("Context Packs com memória", () => {
  const pack: ContextPack = {
    plan: {
      schemaVersion: 2,
      id: "0190aaaa-bbbb-7ccc-8ddd-eeeeffff0001",
      projectId: 1,
      projectName: "acme",
      task: "regras de dinheiro",
      createdAt: NOW,
      indexGeneration: 3,
      headRef: null,
      headCommit: null,
      analysis: { terms: ["dinheiro"], expandedTerms: [], intents: [], mentionedPaths: [] },
      summary: "0 fonte(s). 1 memória(s) confirmada(s) do projeto.",
      sources: [],
      omitted: [],
      risks: [],
      limitations: [],
      tokenBudget: 8000,
      estimatedTokens: 30,
      memories: [
        {
          memoryId: CONFIRMED_ID,
          revision: 2,
          kind: "decision",
          title: "Dinheiro em centavos",
          score: 1,
          reasons: ["Texto da memória corresponde à tarefa"],
          estimatedTokens: 30,
        },
      ],
    },
    excerpts: [],
    staleSources: 0,
    memories: [
      {
        reference: {
          memoryId: CONFIRMED_ID,
          revision: 2,
          kind: "decision",
          title: "Dinheiro em centavos",
          score: 1,
          reasons: ["Texto da memória corresponde à tarefa"],
          estimatedTokens: 30,
        },
        state: "revised",
        current: memory({
          id: CONFIRMED_ID,
          status: "confirmed",
          title: "Dinheiro em centavos",
          body: "Inteiros em centavos.",
          revision: 3,
        }),
      },
    ],
    issues: [issue],
    recentChanges: [],
  };

  it("mostra memórias e contradições e exclui memória do export", async () => {
    const user = userEvent.setup();
    const { calls } = baseIpc({
      list_context_packs: () => [
        {
          id: pack.plan.id,
          projectId: 1,
          task: pack.plan.task,
          createdAt: NOW,
          indexGeneration: 3,
          fileCount: 0,
          estimatedTokens: 30,
          tokenBudget: 8000,
        },
      ],
      get_context_pack: () => pack,
      render_context_pack: () => "# md",
    });
    renderApp();
    await open(user, "Context Packs");
    await user.click(await screen.findByRole("button", { name: /regras de dinheiro/ }));
    expect(await screen.findByText("Memória do projeto (confirmada)")).toBeInTheDocument();
    expect(screen.getByText("Editada depois do pack")).toBeInTheDocument();
    expect(screen.getByText("Inteiros em centavos.")).toBeInTheDocument();
    expect(screen.getByRole("article", { name: issue.title })).toBeInTheDocument();

    await user.click(
      screen.getByRole("checkbox", { name: "Incluir no export: Dinheiro em centavos" }),
    );
    expect(screen.getByText(/~0 de 8000 tokens estimados/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Copiar Markdown" }));
    await waitFor(() =>
      expect(calls).toContainEqual({
        cmd: "render_context_pack",
        args: {
          packId: pack.plan.id,
          format: "markdown",
          options: { excludedPaths: [], excludedMemories: [CONFIRMED_ID] },
        },
      }),
    );
  });
});

describe("Exportar memória para o repositório", () => {
  const preview = (token: string) => ({
    directory: ".nexus/memory",
    items: [
      {
        relPath: ".nexus/memory/sessoes-no-servidor-0000c0f1.md",
        memoryId: CONFIRMED_ID,
        title: "Sessões no servidor",
        action: "create" as const,
      },
      {
        relPath: ".nexus/memory/dinheiro-em-centavos-0000c0f2.md",
        memoryId: "01a13000-0000-7000-8000-00000000c0f2",
        title: "Dinheiro em centavos",
        action: "unchanged" as const,
      },
    ],
    obsolete: [{ relPath: ".nexus/memory/nota-antiga.md", title: null }],
    token,
  });

  it("mostra a prévia e só grava com o clique, usando o token da prévia", async () => {
    const user = userEvent.setup();
    const { calls } = baseIpc({
      preview_memory_export: () => preview("t-1"),
      export_memory: () => ({
        directory: ".nexus/memory",
        created: 1,
        updated: 0,
        unchanged: 1,
      }),
    });
    renderApp();
    await open(user, "Memória");
    await user.click(await screen.findByRole("button", { name: "Exportar para o repositório" }));
    const dialog = await screen.findByRole("dialog");
    const list = await within(dialog).findByRole("list", { name: "Arquivos da exportação" });
    expect(within(list).getByText("Sessões no servidor")).toBeInTheDocument();
    expect(within(list).getByText("criar")).toBeInTheDocument();
    expect(within(list).getByText("sem mudança")).toBeInTheDocument();
    expect(within(dialog).getByText(".nexus/memory/nota-antiga.md")).toBeInTheDocument();
    expect(calls.some((c) => c.cmd === "export_memory")).toBe(false);

    await user.click(within(dialog).getByRole("button", { name: "Gravar 1 arquivo" }));
    await waitFor(() => {
      expect(calls).toContainEqual({ cmd: "export_memory", args: { projectId: 1, token: "t-1" } });
    });
    expect(await screen.findByText("Memória exportada")).toBeInTheDocument();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("com tudo igual no repositório, não há o que gravar", async () => {
    const user = userEvent.setup();
    baseIpc({
      preview_memory_export: () => ({
        ...preview("t-2"),
        items: preview("t-2").items.map((i) => ({ ...i, action: "unchanged" as const })),
        obsolete: [],
      }),
    });
    renderApp();
    await open(user, "Memória");
    await user.click(await screen.findByRole("button", { name: "Exportar para o repositório" }));
    const dialog = await screen.findByRole("dialog");
    expect(
      await within(dialog).findByText("Tudo já está igual no repositório."),
    ).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Gravar 0 arquivos" })).toBeDisabled();
  });

  it("se a memória mudou depois da prévia, mostra o erro e recarrega a prévia", async () => {
    const user = userEvent.setup();
    let version = 1;
    const { calls } = baseIpc({
      preview_memory_export: () => preview(`t-${version}`),
      export_memory: () => {
        version = 2;
        throw new IpcError({
          code: "conflict",
          message: "a memória ou a pasta mudou desde a prévia",
          opId: null,
        });
      },
    });
    renderApp();
    await open(user, "Memória");
    await user.click(await screen.findByRole("button", { name: "Exportar para o repositório" }));
    const dialog = await screen.findByRole("dialog");
    await user.click(await within(dialog).findByRole("button", { name: "Gravar 1 arquivo" }));
    expect(await within(dialog).findByText(/mudou desde a prévia/)).toBeInTheDocument();
    await waitFor(() => {
      expect(calls.filter((c) => c.cmd === "preview_memory_export").length).toBeGreaterThan(1);
    });
  });
});
