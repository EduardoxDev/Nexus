import { existsSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { expect, test, type Page } from "@playwright/test";

import { copyFixture, createDataDir, launchApp, runCli, type RunningApp } from "./app";

test.skip(process.platform !== "win32", "E2E via CDP depende do WebView2 (Windows)");

let data: ReturnType<typeof createDataDir>;
let repo: ReturnType<typeof copyFixture>;
let app: RunningApp | undefined;

test.beforeEach(() => {
  data = createDataDir();
  repo = copyFixture("ts-webapp", "shopfront");
});

test.afterEach(async () => {
  await app?.close().catch(() => undefined);
  app = undefined;
  data.cleanup();
  repo.cleanup();
});

async function openProjects(page: Page) {
  await page
    .getByRole("navigation", { name: "Navegação principal" })
    .getByRole("button", { name: "Projetos" })
    .click();
}

function projectRow(page: Page) {
  return page.getByRole("complementary", { name: "Projetos autorizados" }).getByRole("button", {
    name: /shopfront/,
  });
}

test("indexa o projeto ao abrir, reage a alterações e remove sem apagar arquivos", async () => {
  // Registrado pela CLI (mesmo banco): o app faz a passada inicial ao abrir.
  runCli(data.dir, ["projects", "add", repo.root]);
  app = await launchApp(data.dir);
  const { page } = app;
  await openProjects(page);

  await expect(page.getByRole("heading", { name: "shopfront", level: 2 })).toBeVisible();
  await expect(projectRow(page)).toContainText("13 arquivos", { timeout: 20_000 });
  await expect(page.getByText("Não é um repositório git")).toBeVisible();
  await expect(page.getByRole("list", { name: "Linguagens" })).toContainText("typescript");
  await expect(page.getByText(".github/workflows/ci.yml")).toBeVisible();

  // Alteração no disco ⇒ monitor de arquivos ⇒ indexação incremental automática.
  writeFileSync(
    join(repo.root, "src", "checkout", "coupon.ts"),
    "export const applyCoupon = (cents: number) => Math.round(cents * 0.9);\n",
  );
  await expect(projectRow(page)).toContainText("14 arquivos", { timeout: 20_000 });
  await expect(page.getByRole("cell", { name: /Alteração detectada/ }).first()).toBeVisible();

  // Atualização manual registra um job do usuário.
  await page.getByRole("button", { name: "Atualizar índice" }).click();
  await expect(page.getByRole("cell", { name: /^Você · incremental/ }).first()).toBeVisible({
    timeout: 20_000,
  });

  // Remoção só após confirmação; os arquivos continuam no disco.
  await page.getByRole("button", { name: "Remover do índice" }).click();
  const dialog = page.getByRole("dialog", { name: 'Remover "shopfront" do índice?' });
  await expect(dialog).toContainText("Nenhum arquivo da pasta será alterado");
  await dialog.getByRole("button", { name: "Remover do índice" }).click();
  await expect(page.getByText("Nenhum projeto autorizado")).toBeVisible();
  expect(existsSync(join(repo.root, "src", "auth", "session.ts"))).toBe(true);
  expect(existsSync(join(repo.root, "src", "checkout", "coupon.ts"))).toBe(true);
});

test("arquivos sensíveis aparecem como ignorados, sem conteúdo", async () => {
  writeFileSync(join(repo.root, ".env"), "API_TOKEN=valor-que-nao-pode-vazar\n");
  runCli(data.dir, ["projects", "add", repo.root, "--index"]);
  app = await launchApp(data.dir);
  const { page } = app;
  await openProjects(page);
  const skipped = page.getByRole("region", { name: "Arquivos não indexados" });
  await expect(skipped).toContainText(".env");
  await expect(skipped).toContainText("Sensível");
  await expect(skipped).not.toContainText("valor-que-nao-pode-vazar");
});
