/**
 * Inicia o executável real do NEXUS e conecta o Playwright ao WebView2 via CDP.
 * Cada teste usa um diretório de dados temporário (NEXUS_DATA_DIR) — nada toca os dados do usuário.
 */
import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import { cpSync, existsSync, mkdtempSync, rmSync } from "node:fs";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import { chromium, type Browser, type Page } from "@playwright/test";

/** Executável com o frontend embutido (gerado por `tauri build --debug --no-bundle`). */
export const APP_PATH =
  process.env.NEXUS_E2E_APP ??
  resolve(import.meta.dirname, "../../../target/debug/nexus-desktop.exe");

/** CLI `nexus` (mesmo banco do app); usada para registrar projetos sem o seletor nativo. */
export const CLI_PATH =
  process.env.NEXUS_E2E_CLI ?? resolve(import.meta.dirname, "../../../target/debug/nexus.exe");

export function runCli(dataDir: string, args: string[]): unknown {
  const result = spawnSync(CLI_PATH, ["--data-dir", dataDir, "--json", ...args], {
    encoding: "utf8",
  });
  if (result.status !== 0) {
    throw new Error(`nexus ${args.join(" ")} falhou (${result.status}): ${result.stderr}`);
  }
  return JSON.parse(result.stdout) as unknown;
}

const FIXTURES = resolve(import.meta.dirname, "../../../tests/fixtures");

/** Copia uma fixture para uma pasta temporária própria, com o nome de pasta indicado. */
export function copyFixture(
  fixture: string,
  folderName: string,
): { root: string; cleanup: () => void } {
  const parent = mkdtempSync(join(tmpdir(), "nexus-e2e-repo-"));
  const root = join(parent, folderName);
  cpSync(join(FIXTURES, fixture), root, { recursive: true });
  return { root, cleanup: () => rmSync(parent, { recursive: true, force: true }) };
}

export function createDataDir(): { dir: string; cleanup: () => void } {
  const dir = mkdtempSync(join(tmpdir(), "nexus-e2e-"));
  return { dir, cleanup: () => rmSync(dir, { recursive: true, force: true }) };
}

async function freePort(): Promise<number> {
  return new Promise((ok, fail) => {
    const server = createServer();
    server.once("error", fail);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close(() => {
        if (address && typeof address === "object") ok(address.port);
        else fail(new Error("porta indisponível"));
      });
    });
  });
}

async function waitForCdp(port: number, child: ChildProcess, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (child.exitCode !== null) throw new Error(`o app terminou com código ${child.exitCode}`);
    try {
      const res = await fetch(`http://127.0.0.1:${port}/json/version`);
      if (res.ok) return;
    } catch {
      // ainda iniciando
    }
    await new Promise((r) => setTimeout(r, 250));
  }
  throw new Error(`CDP não respondeu na porta ${port} em ${timeoutMs} ms`);
}

export interface RunningApp {
  page: Page;
  browser: Browser;
  child: ChildProcess;
  /** Fecha a janela como o usuário faria (WM_CLOSE) e espera o processo sair. */
  close: () => Promise<void>;
}

export async function launchApp(dataDir: string): Promise<RunningApp> {
  if (!existsSync(APP_PATH)) {
    throw new Error(
      `executável não encontrado em ${APP_PATH}. Rode "pnpm tauri build --debug --no-bundle" antes.`,
    );
  }
  const port = await freePort();
  const child = spawn(APP_PATH, [], {
    env: {
      ...process.env,
      NEXUS_DATA_DIR: dataDir,
      NEXUS_LOG: "info",
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
    },
    stdio: "ignore",
  });
  await waitForCdp(port, child, 30_000);
  const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  const context = browser.contexts()[0];
  if (!context) throw new Error("nenhum contexto CDP encontrado");
  const page = context.pages()[0] ?? (await context.waitForEvent("page"));
  await page.waitForLoadState("domcontentloaded");

  const close = async () => {
    await browser.close().catch(() => undefined);
    if (child.exitCode !== null) return;
    const exited = new Promise<void>((ok) => child.once("exit", () => ok()));
    if (process.platform === "win32" && child.pid !== undefined) {
      // Sem /F: envia WM_CLOSE, o mesmo que clicar no X da janela.
      spawnSync("taskkill", ["/PID", String(child.pid)], { stdio: "ignore" });
    } else {
      child.kill("SIGTERM");
    }
    const timeout = new Promise<"timeout">((ok) => setTimeout(() => ok("timeout"), 10_000));
    if ((await Promise.race([exited, timeout])) === "timeout") {
      child.kill();
      throw new Error("o app não fechou em 10 s após WM_CLOSE");
    }
  };
  return { page, browser, child, close };
}
