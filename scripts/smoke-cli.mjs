// Fumaça da CLI que vai dentro do instalador: `--version` e um handshake MCP real
// (initialize → initialized → tools/list) numa pasta de dados vazia.
// Uso: node scripts/smoke-cli.mjs <caminho da CLI> [versão esperada]
import { spawn, spawnSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const [cli, expectedVersion] = process.argv.slice(2);
if (!cli) {
  console.error("uso: node scripts/smoke-cli.mjs <cli> [versão]");
  process.exit(2);
}

function fail(message) {
  console.error(`FALHOU: ${message}`);
  process.exit(1);
}

const version = spawnSync(cli, ["--version"], { encoding: "utf8", timeout: 20_000 });
if (version.status !== 0) fail(`--version saiu com ${version.status}: ${version.stderr}`);
const printed = version.stdout.trim();
if (!/^nexus \d+\.\d+\.\d+/.test(printed)) fail(`--version inesperado: ${printed}`);
if (expectedVersion && !printed.endsWith(expectedVersion.replace(/^v/, "")))
  fail(`versão ${printed}, esperada ${expectedVersion}`);
console.log(`ok: ${printed}`);

const dataDir = mkdtempSync(join(tmpdir(), "nexus-smoke-"));
const server = spawn(cli, ["--data-dir", dataDir, "mcp", "serve"], {
  stdio: ["pipe", "pipe", "inherit"],
});
const timer = setTimeout(() => {
  server.kill();
  fail("o servidor MCP não respondeu em 30 s");
}, 30_000);

const send = (msg) => server.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", ...msg })}\n`);
let buffer = "";
server.stdout.setEncoding("utf8");
server.stdout.on("data", (chunk) => {
  buffer += chunk;
  let nl;
  while ((nl = buffer.indexOf("\n")) >= 0) {
    const line = buffer.slice(0, nl).trim();
    buffer = buffer.slice(nl + 1);
    if (!line) continue;
    const msg = JSON.parse(line);
    if (msg.error) fail(`erro MCP: ${JSON.stringify(msg.error)}`);
    if (msg.id === 1) {
      console.log(`ok: initialize (${msg.result.serverInfo?.name ?? "?"})`);
      send({ method: "notifications/initialized" });
      send({ id: 2, method: "tools/list" });
    } else if (msg.id === 2) {
      const tools = msg.result.tools ?? [];
      if (tools.length === 0) fail("tools/list veio vazio");
      console.log(`ok: tools/list com ${tools.length} ferramentas`);
      clearTimeout(timer);
      // Fechar a entrada encerra o servidor; a pasta só é apagada depois que ele solta o banco.
      server.once("exit", () => {
        try {
          rmSync(dataDir, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
        } catch {
          // pasta temporária: o sistema limpa depois
        }
        process.exit(0);
      });
      server.stdin.end();
      setTimeout(() => server.kill(), 5_000).unref();
    }
  }
});
server.on("error", (e) => fail(`não foi possível iniciar: ${e.message}`));

send({
  id: 1,
  method: "initialize",
  params: {
    protocolVersion: "2025-06-18",
    capabilities: {},
    clientInfo: { name: "nexus-smoke", version: "0" },
  },
});
