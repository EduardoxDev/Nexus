import { expect, test } from "@playwright/test";

import { createDataDir, launchApp, type RunningApp } from "./app";

test.skip(process.platform !== "win32", "E2E via CDP depende do WebView2 (Windows)");

let data: ReturnType<typeof createDataDir>;
let app: RunningApp | undefined;

test.beforeEach(() => {
  data = createDataDir();
});

test.afterEach(async () => {
  await app?.close().catch(() => undefined);
  app = undefined;
  data.cleanup();
});

test("abre na Visão geral com o banco migrado para o schema atual", async () => {
  app = await launchApp(data.dir);
  const { page } = app;
  await expect(page.getByRole("heading", { name: "Visão geral" })).toBeVisible();
  await expect(page.getByText("Banco OK · schema v2")).toBeVisible();
  await expect(page.getByText("Migrations aplicadas nesta inicialização: v1, v2")).toBeVisible();
});

test("seções de fases futuras ficam desabilitadas", async () => {
  app = await launchApp(data.dir);
  const nav = app.page.getByRole("navigation", { name: "Navegação principal" });
  const search = nav.getByRole("button", { name: /Busca/ });
  await expect(search).toHaveAttribute("aria-disabled", "true");
  // Playwright trata aria-disabled como desabilitado; força o clique como um usuário faria.
  await search.click({ force: true });
  await expect(app.page.getByRole("heading", { name: "Visão geral" })).toBeVisible();
});

test("Diagnóstico mostra dados reais e executa a verificação de integridade", async () => {
  app = await launchApp(data.dir);
  const { page } = app;
  await page
    .getByRole("navigation", { name: "Navegação principal" })
    .getByRole("button", { name: "Diagnóstico" })
    .click();
  await expect(page.getByRole("cell", { name: "foundation" })).toBeVisible();
  await expect(page.getByText(/FTS5 Disponível/)).toBeVisible();
  await expect(page.getByText(data.dir, { exact: false }).first()).toBeVisible();

  await page.getByRole("button", { name: "Verificar integridade" }).click();
  await expect(page.getByText(/Nenhum problema encontrado/)).toBeVisible();
  await expect(
    page.getByRole("cell", { name: "Verificação de integridade" }).first(),
  ).toBeVisible();
});

test("paleta de comandos abre com Ctrl+K e navega", async () => {
  app = await launchApp(data.dir);
  const { page } = app;
  await expect(page.getByRole("heading", { name: "Visão geral" })).toBeVisible();
  await page.keyboard.press("Control+k");
  const palette = page.getByRole("dialog", { name: "Paleta de comandos" });
  await expect(palette).toBeVisible();
  await palette.getByRole("combobox").fill("Diagnóstico");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Diagnóstico", level: 1 })).toBeVisible();
});

test("tema e movimento reduzido persistem após fechar e reabrir o app", async () => {
  app = await launchApp(data.dir);
  let { page } = app;
  await page
    .getByRole("navigation", { name: "Navegação principal" })
    .getByRole("button", { name: "Configurações" })
    .click();
  await page.getByRole("radio", { name: "Claro" }).click();
  await page.getByRole("radio", { name: "Reduzir" }).click();
  await expect(page.locator("html")).toHaveClass(/light/);
  await expect(page.locator("html")).toHaveAttribute("data-reduced-motion", "true");

  await app.close();
  app = await launchApp(data.dir);
  page = app.page;

  await expect(page.getByRole("heading", { name: "Visão geral" })).toBeVisible();
  await expect(page.locator("html")).toHaveClass(/light/);
  await expect(page.locator("html")).toHaveAttribute("data-reduced-motion", "true");
  // Segunda abertura: nenhuma migration pendente.
  await expect(page.getByText(/Migrations aplicadas nesta inicialização/)).toHaveCount(0);
  await page
    .getByRole("navigation", { name: "Navegação principal" })
    .getByRole("button", { name: "Configurações" })
    .click();
  await expect(page.getByRole("radio", { name: "Claro" })).toBeChecked();
  await expect(page.getByRole("radio", { name: "Reduzir" })).toBeChecked();
});
