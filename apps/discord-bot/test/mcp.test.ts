/**
 * Cliente MCP contra o `nexus mcp serve` real, sobre uma cópia indexada da fixture saas-app.
 */
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import { afterAll, beforeAll, describe, expect, it } from "vitest";

import { McpClient, McpToolError, McpUnavailableError } from "../src/mcp/client.ts";

const ROOT = resolve(import.meta.dirname, "../../..");
const CLI = join(ROOT, "target", "debug", process.platform === "win32" ? "nexus.exe" : "nexus");

describe.skipIf(!existsSync(CLI))("McpClient contra nexus mcp serve", () => {
  let dir: string;
  let client: McpClient;

  beforeAll(() => {
    dir = mkdtempSync(join(tmpdir(), "nexus-bot-mcp-"));
    const repo = join(dir, "saas-app");
    cpSync(join(ROOT, "tests", "fixtures", "saas-app"), repo, {
      recursive: true,
    });
    const data = join(dir, "data");
    const r = spawnSync(CLI, ["--data-dir", data, "projects", "add", repo, "--index"], {
      encoding: "utf8",
    });
    if (r.status !== 0) throw new Error(`falha ao indexar a fixture: ${r.stderr}`);
    client = new McpClient({
      command: CLI,
      args: ["--data-dir", data, "mcp", "serve"],
    });
  });

  afterAll(async () => {
    await client?.close();
    rmSync(dir, { recursive: true, force: true });
  });

  it("lists the indexed project", async () => {
    const r = await client.callTool<{
      projects: { name: string; indexedFiles: number }[];
    }>("list_projects", {});
    expect(r.projects[0]?.name).toBe("saas-app");
    expect(r.projects[0]?.indexedFiles).toBeGreaterThan(0);
  });

  it("searches with sources", async () => {
    const r = await client.callTool<{
      hits: { relPath: string; startLine: number }[];
    }>("search_project", { query: "cálculo de frete", limit: 3 });
    expect(r.hits[0]?.relPath).toBe("src/checkout/shipping.ts");
    expect(r.hits[0]?.startLine).toBeGreaterThanOrEqual(1);
  });

  it("tool errors become McpToolError", async () => {
    await expect(
      client.callTool("search_project", { query: "x", project: "nao-existe" }),
    ).rejects.toBeInstanceOf(McpToolError);
  });

  it("restarts reliably right after a kill (no fight for the database lock)", async () => {
    for (let n = 0; n < 25; n++) {
      client.__killForTests();
      const r = await client.callTool<{ projects: unknown[] }>("list_projects", {});
      expect(r.projects.length).toBe(1);
    }
  });

  it("restarts after the server exits", async () => {
    client.__killForTests();
    const r = await client.callTool<{ projects: unknown[] }>("list_projects", {});
    expect(r.projects.length).toBe(1);
  });
});

/** Servidor MCP falso: responde ao handshake e devolve as variáveis de ambiente que recebeu. */
const ENV_SERVER = `
const rl = require("node:readline").createInterface({ input: process.stdin });
rl.on("line", (line) => {
  const m = JSON.parse(line);
  if (m.id === undefined) return;
  const result = m.method === "initialize"
    ? { protocolVersion: "2025-11-25", capabilities: {}, serverInfo: { name: "fake", version: "0" } }
    : { structuredContent: { keys: Object.keys(process.env) } };
  process.stdout.write(JSON.stringify({ jsonrpc: "2.0", id: m.id, result }) + String.fromCharCode(10));
});
`;

describe("McpClient e o ambiente do processo", () => {
  it("never passes Discord secrets to the NEXUS process", async () => {
    process.env.DISCORD_TOKEN = "segredo-de-teste";
    try {
      const fake = new McpClient({ command: process.execPath, args: ["-e", ENV_SERVER] });
      const r = await fake.callTool<{ keys: string[] }>("x", {});
      await fake.close();
      expect(r.keys).not.toContain("DISCORD_TOKEN");
      expect(r.keys.length).toBeGreaterThan(0);
    } finally {
      delete process.env.DISCORD_TOKEN;
    }
  });
});

describe("McpClient sem servidor útil", () => {
  it("times out", async () => {
    const stuck = new McpClient({
      command: process.execPath,
      args: ["-e", "setInterval(() => {}, 1e9)"],
      timeoutMs: 300,
    });
    const started = Date.now();
    await expect(stuck.callTool("list_projects", {})).rejects.toBeInstanceOf(McpUnavailableError);
    expect(Date.now() - started).toBeLessThan(1_000);
    await stuck.close();
  });
});
