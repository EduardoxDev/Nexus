import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import type { Settings, UpdateStatus } from "@/lib/bindings";
import { defaultSettings, renderApp, setupIpc } from "@/test/harness";

/** Configurações mutáveis dentro do teste (o `set_setting` simulado grava aqui). */
function withSettings(initial: Partial<Settings>, check: () => UpdateStatus) {
  let settings: Settings = { ...defaultSettings, ...initial };
  return setupIpc({
    get_settings: () => settings,
    set_setting: (args) => {
      if (args.key === "updates.check")
        settings = { ...settings, updateCheck: args.value as Settings["updateCheck"] };
      return settings;
    },
    check_for_update: () => check(),
    install_update: () => null,
  });
}

const available: UpdateStatus = {
  status: "available",
  version: "0.2.1",
  currentVersion: "0.2.0",
  notes: "Correções na busca.",
  date: null,
};

describe("atualizações", () => {
  it("pergunta na primeira abertura e, com 'Agora não', nunca consulta a rede", async () => {
    const user = userEvent.setup();
    const { calls } = withSettings({ updateCheck: "ask" }, () => available);
    renderApp();
    const consent = await screen.findByRole("region", { name: "Verificar atualizações?" });
    expect(within(consent).getByText(/envia seu IP e a versão do app/)).toBeInTheDocument();
    await user.click(within(consent).getByRole("button", { name: "Agora não" }));
    await waitFor(() =>
      expect(screen.queryByRole("region", { name: "Verificar atualizações?" })).toBeNull(),
    );
    expect(calls).toContainEqual({
      cmd: "set_setting",
      args: { key: "updates.check", value: "off" },
    });
    expect(calls.some((c) => c.cmd === "check_for_update")).toBe(false);
  });

  it("com permissão, verifica ao abrir e avisa da versão nova", async () => {
    const { calls } = withSettings({ updateCheck: "on" }, () => available);
    renderApp();
    expect(await screen.findByText("NEXUS 0.2.1 disponível")).toBeInTheDocument();
    expect(calls).toContainEqual({ cmd: "check_for_update", args: { manual: false } });
    expect(screen.queryByRole("region", { name: "Verificar atualizações?" })).toBeNull();
  });

  it("'Verificar agora' em Configurações mostra a versão nova e instala", async () => {
    const user = userEvent.setup();
    const { calls } = withSettings({ updateCheck: "off" }, () => available);
    renderApp();
    const nav = await screen.findByRole("navigation", { name: "Navegação principal" });
    await user.click(within(nav).getByRole("button", { name: /Configurações/ }));
    await user.click(await screen.findByRole("button", { name: "Verificar agora" }));
    const found = await screen.findByRole("region", { name: "Versão 0.2.1 disponível" });
    expect(within(found).getByText("Correções na busca.")).toBeInTheDocument();
    expect(calls).toContainEqual({ cmd: "check_for_update", args: { manual: true } });
    await user.click(within(found).getByRole("button", { name: "Instalar e reiniciar" }));
    await waitFor(() => expect(calls.some((c) => c.cmd === "install_update")).toBe(true));
  });

  it("'Verificar agora' sem novidade diz que está na versão mais recente", async () => {
    const user = userEvent.setup();
    withSettings({ updateCheck: "off" }, () => ({
      status: "upToDate",
      currentVersion: "0.2.0",
    }));
    renderApp();
    const nav = await screen.findByRole("navigation", { name: "Navegação principal" });
    await user.click(within(nav).getByRole("button", { name: /Configurações/ }));
    await user.click(await screen.findByRole("button", { name: "Verificar agora" }));
    expect(await screen.findByRole("status")).toHaveTextContent(
      "Você está na versão mais recente (0.2.0).",
    );
  });
});
