import { expect, test, type Page } from "@playwright/test";

import { copyFixture, createDataDir, launchApp, runCli, type RunningApp } from "./app";

test.skip(process.platform !== "win32", "E2E via CDP depende do WebView2 (Windows)");

let data: ReturnType<typeof createDataDir>;
let repo: ReturnType<typeof copyFixture>;
let app: RunningApp | undefined;

test.beforeEach(() => {
  data = createDataDir();
  repo = copyFixture("saas-app", "acme");
  runCli(data.dir, ["projects", "add", repo.root, "--index"]);
});

test.afterEach(async () => {
  await app?.close().catch(() => undefined);
  app = undefined;
  data.cleanup();
  repo.cleanup();
});

async function open(page: Page, name: string) {
  await page
    .getByRole("navigation", { name: "Navegação principal" })
    .getByRole("button", { name })
    .click();
}

test("busca em português encontra o código certo, com motivos e prévia verificada", async () => {
  app = await launchApp(data.dir);
  const { page } = app;
  await open(page, "Busca");
  await page.getByRole("searchbox").fill("cálculo de frete");
  await page.keyboard.press("Enter");

  const results = page.getByRole("list", { name: "Resultados" });
  const first = results.getByRole("listitem").first();
  await expect(first).toContainText("src/checkout/shipping.ts");
  await expect(first).toContainText("Atual");
  await expect(first.getByRole("list", { name: "Motivos" })).toContainText("Caminho corresponde a");
  await expect(page.getByText(/frete → shipping/)).toBeVisible();

  await first.getByRole("button", { name: /src\/checkout\/shipping\.ts/ }).click();
  const preview = page.getByRole("complementary", { name: "Prévia" });
  await expect(preview).toContainText("export function shippingCost");
});

test("Context Pack é compilado, persiste após reiniciar e mostra fontes verificadas", async () => {
  app = await launchApp(data.dir);
  let { page } = app;
  await open(page, "Context Packs");
  await page
    .getByRole("textbox", { name: "Descreva a tarefa" })
    .fill("Adicionar login com Google respeitando a arquitetura de autenticação existente");
  await page.getByRole("button", { name: "Compilar Context Pack" }).click();

  await expect(page.getByRole("heading", { name: "Arquivos principais" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Decisões" })).toBeVisible();
  await expect(page.getByText("src/auth/oauth/provider.ts").first()).toBeVisible();
  await expect(page.getByText("docs/decisions/0003-oauth-providers.md").first()).toBeVisible();
  await expect(page.getByText(/~\d+ de 8000 tokens estimados/)).toBeVisible();

  await app.close();
  app = await launchApp(data.dir);
  page = app.page;
  await open(page, "Context Packs");
  await page.getByRole("button", { name: /Adicionar login com Google/ }).click();
  await expect(page.getByRole("heading", { name: "Convenções" })).toBeVisible();
  await expect(page.getByText("CONTRIBUTING.md").first()).toBeVisible();
});
