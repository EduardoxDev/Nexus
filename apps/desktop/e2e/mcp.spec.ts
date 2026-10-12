import { spawnSync } from "node:child_process";

import { expect, test } from "@playwright/test";

import { CLI_PATH, copyFixture, createDataDir, launchApp, runCli, type RunningApp } from "./app";

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

/** Uma sessão MCP completa pelo binário real (`nexus mcp serve`), como um agente faria. */
function mcpSession(dataDir: string, messages: object[]): Record<string, unknown>[] {
  const input = messages.map((m) => JSON.stringify(m)).join("\n") + "\n";
  const result = spawnSync(CLI_PATH, ["--data-dir", dataDir, "mcp", "serve"], {
    input,
    encoding: "utf8",
  });
  if (result.status !== 0) throw new Error(`mcp serve falhou: ${result.stderr}`);
  return result.stdout
    .trim()
    .split("\n")
    .map((l) => JSON.parse(l) as Record<string, unknown>);
}

test("agente via MCP busca no mesmo banco do app e a chamada aparece em Integrações", async () => {
  app = await launchApp(data.dir);
  const { page } = app;
  await page
    .getByRole("navigation", { name: "Navegação principal" })
    .getByRole("button", { name: /Integrações/ })
    .click();
  await expect(page.getByText("Nenhuma chamada ainda")).toBeVisible();

  const meta = { "io.modelcontextprotocol/protocolVersion": "2026-07-28" };
  const [discover, search] = mcpSession(data.dir, [
    { jsonrpc: "2.0", id: 1, method: "server/discover", params: { _meta: meta } },
    {
      jsonrpc: "2.0",
      id: 2,
      method: "tools/call",
      params: {
        _meta: meta,
        name: "search_project",
        arguments: { query: "cálculo de frete", limit: 3 },
      },
    },
  ]);
  expect(JSON.stringify(discover)).toContain("2026-07-28");
  expect(JSON.stringify(search)).toContain("src/checkout/shipping.ts");

  const calls = page.getByRole("list", { name: "Chamadas recentes" });
  await expect(calls).toContainText("search_project", { timeout: 15_000 });
  await expect(calls).toContainText("OK");
});
