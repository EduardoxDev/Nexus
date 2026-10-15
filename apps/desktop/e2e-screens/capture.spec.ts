/**
 * Capturas do README, reproduzíveis: o executável real sobre a fixture `saas-app`, com dados
 * preparados pela CLI (projeto indexado, ADRs importados, uma decisão confirmada).
 *
 *   pnpm e2e:build && pnpm screens         # grava em docs/assets/
 *   NEXUS_SCREENS_DIR=C:\tmp\shots pnpm screens
 */
import { mkdirSync } from "node:fs";
import { join, resolve } from "node:path";

import { expect, test, type Page } from "@playwright/test";

import { copyFixture, createDataDir, launchApp, runCli, type RunningApp } from "../e2e/app";

test.skip(process.platform !== "win32", "capturas via CDP dependem do WebView2 (Windows)");

const OUT = process.env.NEXUS_SCREENS_DIR ?? resolve(import.meta.dirname, "../../../docs/assets");
const VIEWPORT = { width: 1440, height: 900 };

let data: ReturnType<typeof createDataDir>;
let repo: ReturnType<typeof copyFixture>;
let app: RunningApp | undefined;

test.beforeAll(() => {
  mkdirSync(OUT, { recursive: true });
  data = createDataDir();
  repo = copyFixture("saas-app", "acme");
  runCli(data.dir, ["projects", "add", repo.root, "--index"]);
  runCli(data.dir, ["memory", "import-adrs"]);
  const list = runCli(data.dir, ["memory", "list"]) as { id: string; title: string }[];
  const money = list.find((m) => m.title.includes("centavos"));
  if (money) runCli(data.dir, ["memory", "confirm", money.id]);
  runCli(data.dir, ["freshness", "scan"]);
});

test.afterAll(async () => {
  await app?.close().catch(() => undefined);
  data.cleanup();
  repo.cleanup();
});

async function open(page: Page, name: string) {
  await page
    .getByRole("navigation", { name: "Navegação principal" })
    .getByRole("button", { name })
    .click();
}

async function shot(page: Page, name: string) {
  // Sem cursor piscando nem animações em andamento na imagem.
  await page
    .locator("body")
    .click({ position: { x: 1, y: 1 }, force: true })
    .catch(() => undefined);
  // Espera as animações de entrada e a contagem dos indicadores (600 ms) terminarem.
  await page.waitForTimeout(1000);
  await page.screenshot({ path: join(OUT, `${name}.png`), animations: "disabled" });
}

async function setTheme(page: Page, label: "Escuro" | "Claro") {
  await open(page, "Configurações");
  await page.getByRole("radio", { name: label }).click();
  await expect(page.getByRole("radio", { name: label })).toBeChecked();
}

test("capturas do README", async () => {
  test.setTimeout(180_000);
  app = await launchApp(data.dir);
  const { page } = app;
  await page.setViewportSize(VIEWPORT);

  await setTheme(page, "Escuro");
  await open(page, "Visão geral");
  await expect(page.getByText("Conectando ideias, criando o amanhã.")).toBeVisible();
  await shot(page, "visao-geral-escuro");

  await open(page, "Busca");
  await page.getByRole("searchbox").fill("cálculo de frete");
  await page.keyboard.press("Enter");
  const first = page.getByRole("list", { name: "Resultados" }).getByRole("listitem").first();
  await expect(first).toContainText("src/checkout/shipping.ts");
  await first.getByRole("button", { name: /src\/checkout\/shipping\.ts/ }).click();
  await expect(page.getByRole("complementary", { name: "Prévia" })).toContainText(
    "export function shippingCost",
  );
  await shot(page, "busca-com-previa");

  await open(page, "Context Packs");
  await page
    .getByRole("textbox", { name: "Descreva a tarefa" })
    .fill("Adicionar login com Google respeitando a arquitetura de autenticação existente");
  await page.getByRole("button", { name: "Compilar Context Pack" }).click();
  await expect(page.getByRole("heading", { name: "Arquivos principais" })).toBeVisible();
  await shot(page, "context-pack");

  await open(page, "Memória");
  await page.getByRole("tab", { name: /Confirmadas/ }).click();
  await page.getByRole("button", { name: /Dinheiro em centavos inteiros/ }).click();
  await expect(page.getByText("Histórico de revisões")).toBeVisible();
  await shot(page, "memoria");

  await open(page, "Projetos");
  await page.getByRole("button", { name: /acme/ }).first().click();
  await page.waitForTimeout(500);
  await shot(page, "projeto");

  await open(page, "Mapa");
  await expect(page.getByRole("complementary", { name: /Mais centrais/ })).toBeVisible();
  // O layout de forças para sozinho quando estabiliza.
  await page.waitForTimeout(3000);
  await shot(page, "mapa");

  await open(page, "Integrações");
  await shot(page, "integracoes");

  await setTheme(page, "Claro");
  await open(page, "Visão geral");
  await shot(page, "visao-geral-claro");

  await page.keyboard.press("Control+K");
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.keyboard.type("me");
  await page.waitForTimeout(300);
  await page.screenshot({ path: join(OUT, "paleta-de-comandos.png"), animations: "disabled" });
  await page.keyboard.press("Escape");

  // Deixa o tema como estava para o próximo uso desta pasta de dados.
  await setTheme(page, "Escuro");
});
