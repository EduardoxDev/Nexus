import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { IpcError, operation, renderApp, setupIpc } from "@/test/harness";

describe("shell do app", () => {
  it("mostra a visão geral com dados do core", async () => {
    setupIpc();
    renderApp();
    expect(await screen.findByRole("heading", { name: "Visão geral" })).toBeInTheDocument();
    expect(await screen.findByText("v1 (atual)")).toBeInTheDocument();
    expect(screen.getByText("Banco OK · schema v1")).toBeInTheDocument();
  });

  it("Integrações mostra a configuração de cada agente, as ferramentas e as chamadas MCP", async () => {
    const user = userEvent.setup();
    setupIpc({
      get_integration_info: () => ({
        cliPath: "C:\\NEXUS\\nexus.exe",
        dataDir: "C:\\dados",
        serveArgs: ["--data-dir", "C:\\dados", "mcp", "serve"],
        claudeCommand:
          "claude mcp add nexus -- C:\\NEXUS\\nexus.exe --data-dir C:\\dados mcp serve",
        clients: [
          {
            id: "claude-code",
            name: "Claude Code",
            kind: "command",
            text: "claude mcp add nexus -- C:\\NEXUS\\nexus.exe --data-dir C:\\dados mcp serve",
            whereHint: "Rode uma vez num terminal.",
          },
          {
            id: "cursor",
            name: "Cursor",
            kind: "json",
            text: '{\n  "mcpServers": {}\n}',
            whereHint: "Cole em ~/.cursor/mcp.json.",
          },
        ],
        protocolVersions: ["2026-07-28", "2025-11-25"],
        tools: [
          {
            name: "search_project",
            title: "Search",
            description: "Hybrid search.",
            readOnly: true,
          },
          {
            name: "refresh_project_index",
            title: "Refresh",
            description: "Incremental update.",
            readOnly: false,
          },
        ],
      }),
      list_operations: () => [
        operation({
          kind: "mcp.tool",
          detail: { tool: "search_project", isError: false },
          durationMs: 12,
        }),
      ],
    });
    renderApp();
    const nav = await screen.findByRole("navigation", { name: "Navegação principal" });
    const item = within(nav).getByRole("button", { name: /Integrações/ });
    expect(item).not.toHaveAttribute("aria-disabled");
    await user.click(item);
    expect(
      await screen.findByRole("heading", { name: "Integrações", level: 1 }),
    ).toBeInTheDocument();
    expect(await screen.findByLabelText("Comando para registrar no Claude Code")).toHaveTextContent(
      "claude mcp add nexus -- C:\\NEXUS\\nexus.exe --data-dir C:\\dados mcp serve",
    );
    expect(screen.getByText("Rode uma vez num terminal.")).toBeInTheDocument();
    await user.click(screen.getByRole("radio", { name: "Cursor" }));
    expect(await screen.findByLabelText("Configuração para o Cursor")).toHaveTextContent(
      '"mcpServers"',
    );
    expect(screen.getByText("Cole em ~/.cursor/mcp.json.")).toBeInTheDocument();
    const tools = screen.getByRole("list", { name: "Ferramentas" });
    expect(within(tools).getByText("Somente leitura")).toBeInTheDocument();
    expect(within(tools).getByText("Grava dados locais do NEXUS")).toBeInTheDocument();
    expect(screen.getByText(/Protocolo MCP 2026-07-28/)).toBeInTheDocument();
    const calls = await screen.findByRole("list", { name: "Chamadas recentes" });
    expect(within(calls).getByText("search_project")).toBeInTheDocument();
    expect(within(calls).getByText("OK")).toBeInTheDocument();
  });

  it('mostra a marca e leva de "Como usar" a cada seção', async () => {
    const user = userEvent.setup();
    setupIpc();
    renderApp();
    expect(await screen.findByText("Conectando ideias, criando o amanhã.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Abrir: Registrar a memória do projeto" }));
    const nav = screen.getByRole("navigation", { name: "Navegação principal" });
    await waitFor(() =>
      expect(within(nav).getByRole("button", { name: /Memória/ })).toHaveAttribute(
        "aria-current",
        "page",
      ),
    );
  });

  it("indicadores da Visão geral somam os dados reais de todos os projetos", async () => {
    const user = userEvent.setup();
    const project = (id: number, name: string) => ({
      project: {
        id,
        name,
        rootPath: `C:\\repos\\${name}`,
        createdAt: 0,
        lastOpenedAt: 0,
        lastIndexedAt: Date.now() - 60_000,
        indexGeneration: 1,
        headRef: null,
        headCommit: null,
      },
      rootExists: true,
      counts: {
        total: 14,
        indexed: 13,
        skipped: 1,
        failed: 0,
        indexedBytes: 2048,
        indexedLines: 100,
      },
      activeJob: null,
      lastJob: null,
    });
    setupIpc({
      list_projects: () => [project(1, "loja"), project(2, "api")],
      get_memory_counts: (args) =>
        args.projectId === 1
          ? { proposed: 1, confirmed: 2, archived: 0, superseded: 0 }
          : { proposed: 0, confirmed: 3, archived: 4, superseded: 0 },
      get_freshness_summary: (args) => ({
        projectId: args.projectId as number,
        lastScan: null,
        scanOutdated: false,
        open: args.projectId === 1 ? 3 : 1,
        warnings: 0,
        needsReview: 0,
        contradictions: args.projectId === 1 ? 1 : 0,
        dismissed: 0,
        byKind: [],
      }),
    });
    renderApp();

    const region = await screen.findByRole("region", { name: "Indicadores" });
    const kpi = (name: RegExp) => within(region).getByRole("button", { name });
    // Os números contam até o valor final (animação curta).
    const value = (name: RegExp) => kpi(name).querySelector(".font-display")?.textContent;
    await waitFor(() => expect(value(/^Projetos/)).toBe("2"));
    await waitFor(() => expect(value(/^Arquivos indexados/)).toBe("26"));
    expect(kpi(/^Arquivos indexados/)).toHaveTextContent("200 linhas");
    await waitFor(() => expect(value(/^Memórias confirmadas/)).toBe("5"));
    expect(kpi(/^Memórias confirmadas/)).toHaveTextContent("1 proposta aguardando revisão");
    await waitFor(() => expect(value(/^Alertas abertos/)).toBe("4"));
    expect(kpi(/^Alertas abertos/)).toHaveTextContent("1 contradição a revisar");

    await user.click(kpi(/^Alertas abertos/));
    const nav = screen.getByRole("navigation", { name: "Navegação principal" });
    await waitFor(() =>
      expect(within(nav).getByRole("button", { name: /Atualidade/ })).toHaveAttribute(
        "aria-current",
        "page",
      ),
    );
  });

  it("navega para o Diagnóstico e mostra migrations reais do contrato", async () => {
    const user = userEvent.setup();
    setupIpc();
    renderApp();
    const nav = await screen.findByRole("navigation", { name: "Navegação principal" });
    await user.click(within(nav).getByRole("button", { name: "Diagnóstico" }));
    expect(
      await screen.findByRole("heading", { name: "Diagnóstico", level: 1 }),
    ).toBeInTheDocument();
    expect(await screen.findByText("foundation")).toBeInTheDocument();
    expect(screen.getByText("bbc25775ace5")).toBeInTheDocument();
    expect(screen.getByText(/FTS5 Disponível/)).toBeInTheDocument();
  });

  it("executa a verificação de integridade e mostra o resultado", async () => {
    const user = userEvent.setup();
    const { calls } = setupIpc();
    renderApp();
    const nav = await screen.findByRole("navigation", { name: "Navegação principal" });
    await user.click(within(nav).getByRole("button", { name: "Diagnóstico" }));
    await user.click(await screen.findByRole("button", { name: "Verificar integridade" }));
    expect(await screen.findByText("Nenhum problema encontrado (3 ms).")).toBeInTheDocument();
    expect(calls.some((c) => c.cmd === "run_integrity_check")).toBe(true);
  });

  it("mostra erro com código e opId quando um comando falha", async () => {
    const user = userEvent.setup();
    setupIpc({
      get_diagnostics: () => {
        throw new IpcError({
          code: "storage",
          message: "Falha ao acessar o banco local.",
          opId: "op-42",
        });
      },
    });
    renderApp();
    const nav = await screen.findByRole("navigation", { name: "Navegação principal" });
    await user.click(within(nav).getByRole("button", { name: "Diagnóstico" }));
    const alert = await screen.findByRole("alert");
    expect(within(alert).getByText("Falha ao acessar o banco local.")).toBeInTheDocument();
    expect(within(alert).getByText(/storage · Operação op-42/)).toBeInTheDocument();
  });

  it("abre a paleta com Ctrl+K e navega por ela", async () => {
    const user = userEvent.setup();
    setupIpc();
    renderApp();
    await screen.findByRole("heading", { name: "Visão geral" });
    await user.keyboard("{Control>}k{/Control}");
    const dialog = await screen.findByRole("dialog", { name: "Paleta de comandos" });
    await user.type(within(dialog).getByRole("combobox"), "Configurações");
    await user.keyboard("{Enter}");
    expect(await screen.findByRole("heading", { name: "Configurações" })).toBeInTheDocument();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("troca o tema, persiste via set_setting e aplica a classe no documento", async () => {
    const user = userEvent.setup();
    const { calls } = setupIpc();
    renderApp();
    await screen.findByRole("heading", { name: "Visão geral" });
    await waitFor(() => expect(document.documentElement).toHaveClass("dark"));
    await user.click(screen.getByRole("button", { name: "Alternar tema" }));
    await waitFor(() => expect(document.documentElement).toHaveClass("light"));
    expect(calls).toContainEqual({ cmd: "set_setting", args: { key: "ui.theme", value: "light" } });
  });

  it("mostra operações recentes com rótulos em pt-BR", async () => {
    setupIpc({
      list_operations: () => [operation({ kind: "storage.integrity_check", status: "failed" })],
    });
    renderApp();
    const panel = (await screen.findByRole("heading", { name: "Operações recentes" })).closest(
      "section",
    )!;
    expect(await within(panel).findByText("Verificação de integridade")).toBeInTheDocument();
    expect(within(panel).getByText("Falhou")).toBeInTheDocument();
  });

  it("explica quando o banco foi criado por uma versão mais nova", async () => {
    setupIpc({
      get_startup_status: () => ({
        ready: false,
        dataDir: "C:\\dados",
        error: {
          code: "incompatible_schema",
          message: "o banco está na versão de schema 9",
          opId: null,
        },
        schemaVersion: null,
        appliedMigrations: [],
        backupCreated: false,
        recoveredOperations: 0,
      }),
    });
    renderApp();
    expect(
      await screen.findByRole("heading", { name: "O NEXUS não conseguiu abrir o banco local" }),
    ).toBeInTheDocument();
    expect(screen.getByText(/criado por uma versão mais nova/)).toBeInTheDocument();
    expect(
      screen.queryByRole("navigation", { name: "Navegação principal" }),
    ).not.toBeInTheDocument();
  });
});
