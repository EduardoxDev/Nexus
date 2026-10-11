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

  it("desabilita seções de fases futuras e não navega para elas", async () => {
    const user = userEvent.setup();
    setupIpc();
    renderApp();
    const nav = await screen.findByRole("navigation", { name: "Navegação principal" });
    const search = within(nav).getByRole("button", { name: /Busca/ });
    expect(search).toHaveAttribute("aria-disabled", "true");
    expect(within(search).getByText("Fase 3")).toBeInTheDocument();
    await user.click(search);
    expect(screen.getByRole("heading", { name: "Visão geral" })).toBeInTheDocument();
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
