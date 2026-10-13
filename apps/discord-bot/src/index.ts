/**
 * Inicialização do bot: ambiente, regras de acesso, cliente MCP da NEXUS e Discord.
 * Logs registram só eventos — nunca o token nem conteúdo de mensagens ou do repositório.
 */
import { existsSync } from "node:fs";
import { resolve } from "node:path";

import { Events } from "discord.js";

import { autocompleteProjects, handleNexus } from "./commands/nexus.ts";
import { loadConfig, loadEnv } from "./config.ts";
import { adapt, channelRef, createClient } from "./discord.ts";
import { AlreadyRunningError, acquireInstanceLock } from "./instance.ts";
import { McpClient } from "./mcp/client.ts";

const root = resolve(import.meta.dirname, "..");
const envFile = resolve(root, ".env");
if (existsSync(envFile)) process.loadEnvFile(envFile);

const env = loadEnv(process.env);
const cfg = loadConfig(resolve(root, "bot.config.json"));

let lock: { release(): void };
try {
  lock = acquireInstanceLock(resolve(root, "bot.lock"));
} catch (e) {
  if (!(e instanceof AlreadyRunningError)) throw e;
  console.error(e.message);
  process.exit(1);
}
process.on("exit", () => lock.release());
const mcp = new McpClient({
  command: env.nexusCli,
  args: ["--data-dir", env.nexusDataDir, "mcp", "serve"],
});
const client = createClient();

client.once(Events.ClientReady, (c) => {
  console.log(`NEXUS conectado como ${c.user.tag}`);
});

client.on(Events.InteractionCreate, async (it) => {
  // Só o servidor configurado: fora dele (outros servidores, DMs) o bot não responde.
  if (it.guildId !== env.guildId) return;
  try {
    if (it.isAutocomplete() && it.commandName === "nexus") {
      await it.respond(autocompleteProjects(it.options.getFocused(), channelRef(it), cfg));
    } else if (it.isChatInputCommand() && it.commandName === "nexus") {
      await handleNexus(adapt(it), { tools: mcp, cfg });
    }
  } catch (e) {
    const err = e as Error;
    console.error(`falha ao responder interação: ${err.name}: ${err.message}`);
  }
});

async function shutdown() {
  await mcp.close();
  await client.destroy();
  process.exit(0);
}
process.on("SIGINT", () => void shutdown());
process.on("SIGTERM", () => void shutdown());

await client.login(env.token);
