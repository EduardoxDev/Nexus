import { rmSync } from "node:fs";
import { join } from "node:path";

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

test("ADRs viram propostas; só a confirmada entra no Context Pack e persiste após reiniciar", async () => {
  app = await launchApp(data.dir);
  let { page } = app;
  await open(page, "Memória");
  await page.getByRole("button", { name: "Importar ADRs" }).click();
  await expect(page.getByRole("status")).toContainText("3 proposta(s) nova(s)");
  await expect(page.getByRole("tab", { name: "Propostas (3)" })).toBeVisible();

  await page.getByRole("button", { name: /Dinheiro em centavos inteiros/ }).click();
  await expect(page.getByRole("note")).toContainText("Não entra nos Context Packs");
  await page.getByRole("button", { name: "Confirmar" }).click();
  await expect(page.getByRole("tab", { name: "Confirmadas (1)" })).toBeVisible();

  await open(page, "Context Packs");
  await page
    .getByRole("textbox", { name: "Descreva a tarefa" })
    .fill("formatar valores monetários em centavos na fatura");
  await page.getByRole("button", { name: "Compilar Context Pack" }).click();
  await expect(
    page.getByRole("heading", { name: "Memória do projeto (confirmada)" }),
  ).toBeVisible();
  await expect(
    page.getByText(/Todo valor monetário é um inteiro em centavos/).first(),
  ).toBeVisible();
  // Propostas não confirmadas ficam de fora.
  await expect(page.getByText("ADR 0001 — Sessões no servidor em vez de JWT")).toHaveCount(0);

  await app.close();
  app = await launchApp(data.dir);
  page = app.page;
  await open(page, "Memória");
  await page.getByRole("tab", { name: "Confirmadas (1)" }).click();
  await page.getByRole("button", { name: /Dinheiro em centavos inteiros/ }).click();
  await expect(page.getByText("Histórico de revisões")).toBeVisible();
  await expect(page.getByText(/rev 2/)).toBeVisible();
});

test("arquivo apagado com o app fechado vira alerta na memória, que pode ser dispensado", async () => {
  runCli(data.dir, [
    "memory",
    "add",
    "--kind",
    "convention",
    "--title",
    "Logs pelo logger estruturado",
    "--ref",
    "src/utils/logger.ts",
    "--confirm",
  ]);
  rmSync(join(repo.root, "src/utils/logger.ts"));

  // Ao abrir, o app reindexa o que mudou e reavalia a atualidade sozinho.
  app = await launchApp(data.dir);
  const { page } = app;
  await open(page, "Atualidade");
  const card = page.getByRole("article", { name: /Logs pelo logger estruturado/ });
  await expect(card).toBeVisible({ timeout: 30_000 });
  await expect(card).toContainText("src/utils/logger.ts");
  await expect(card).toContainText("Requer revisão humana");

  await card.getByRole("button", { name: "Dispensar" }).click();
  await card.getByLabel("Motivo (opcional)").fill("logger será recriado");
  await card.getByRole("button", { name: "Dispensar" }).click();
  await expect(page.getByRole("article", { name: /Logs pelo logger estruturado/ })).toHaveCount(0);
  await page.getByRole("radio", { name: "Dispensados" }).click();
  await expect(page.getByRole("article", { name: /Logs pelo logger estruturado/ })).toContainText(
    "logger será recriado",
  );
});
