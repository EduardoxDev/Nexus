/**
 * Inicialização do bot: ambiente, regras de acesso, cliente MCP da NEXUS e Discord.
 * Logs registram só eventos — nunca o token nem conteúdo de mensagens ou do repositório.
 */
import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";

import { Events, MessageFlags } from "discord.js";

import { collectNotices } from "./alerts.ts";
import { handleMod } from "./commands/mod.ts";
import { autocompleteProjects, handleNexus } from "./commands/nexus.ts";
import { loadConfig, loadEnv } from "./config.ts";
import { adapt, adaptMod, channelRef, createClient, messageInfo, sendTo } from "./discord.ts";
import { formatNotices, formatRelease, type BotMessage } from "./format.ts";
import { every } from "./poll.ts";
import { latestRelease, shouldAnnounce } from "./releases.ts";
import { BotState } from "./state.ts";
import { AlreadyRunningError, acquireInstanceLock } from "./instance.ts";
import { McpClient } from "./mcp/client.ts";
import { applyAutomod } from "./moderation/actions.ts";
import { evaluate, FloodTracker } from "./moderation/automod.ts";

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
const moderation = cfg.moderation;
const client = createClient({ automod: moderation?.automod ?? false });
const flood = new FloodTracker();
/** Registro no canal de logs da moderação, se houver. */
const modLog = async (m: BotMessage) => {
  if (!moderation?.logChannelId) return;
  try {
    await sendTo(client, moderation.logChannelId, m);
  } catch (e) {
    console.error(`registro de moderação falhou: ${(e as Error).name}`);
  }
};
// Estado do bot ao lado da pasta de dados da NEXUS dele (não dentro: aquela é da NEXUS).
const state = new BotState(resolve(dirname(env.nexusDataDir), "bot.db"));
const github = () => latestRelease(fetch, cfg.githubRepo);
const POLL_MS = 15 * 60_000;
const stops: (() => void)[] = [];

client.once(Events.ClientReady, (c) => {
  console.log(`NEXUS conectado como ${c.user.tag}`);
  const { announceChannelId, teamAlertsChannelId } = cfg;
  if (announceChannelId) {
    stops.push(
      every(
        POLL_MS,
        async () => {
          const r = await github();
          if (!r) return;
          const seen = shouldAnnounce(state, r);
          if (seen === "first-run")
            console.log(`versão atual no GitHub: ${r.tag} (as próximas serão anunciadas)`);
          if (seen === "new") {
            await sendTo(client, announceChannelId, formatRelease(r));
            console.log(`versão ${r.tag} anunciada`);
          }
        },
        (line) => console.error(`anúncio de versão falhou: ${line}`),
      ),
    );
  }
  if (teamAlertsChannelId && cfg.teamProjects.length > 0) {
    stops.push(
      every(
        POLL_MS,
        async () => {
          const notices = await collectNotices(mcp, cfg.teamProjects, state);
          if (notices.length > 0) await sendTo(client, teamAlertsChannelId, formatNotices(notices));
        },
        (line) => console.error(`avisos da equipe falharam: ${line}`),
        20_000,
      ),
    );
  }
});

client.on(Events.MessageCreate, async (message) => {
  if (!moderation?.automod || message.guildId !== env.guildId) return;
  try {
    const info = messageInfo(message);
    const verdict = evaluate(moderation, info, flood);
    if (!verdict) return;
    const outcome = await applyAutomod(moderation, state, info, verdict, {
      deleteMessage: async () => {
        await message.delete();
      },
      notify: async (userId, text) => {
        if (message.channel.isSendable()) {
          await message.channel.send({ content: text, allowedMentions: { users: [userId] } });
        }
      },
      timeout: async (userId, ms, reason) => {
        const member = message.member ?? (await message.guild?.members.fetch(userId));
        if (!member) throw new Error("membro não encontrado");
        await member.timeout(ms, reason);
      },
      log: modLog,
    });
    console.log(`automod: ${verdict.rule} → ${outcome}`);
  } catch (e) {
    const err = e as Error;
    console.error(`automod falhou: ${err.name}: ${err.message}`);
  }
});

client.on(Events.InteractionCreate, async (it) => {
  // Só o servidor configurado: fora dele (outros servidores, DMs) o bot não responde.
  if (it.guildId !== env.guildId) return;
  try {
    if (it.isAutocomplete() && it.commandName === "nexus") {
      await it.respond(autocompleteProjects(it.options.getFocused(), channelRef(it), cfg));
    } else if (it.isChatInputCommand() && it.commandName === "nexus") {
      await handleNexus(adapt(it), { tools: mcp, cfg, latestRelease: github });
    } else if (it.isChatInputCommand() && it.commandName === "mod") {
      if (!moderation) {
        await it.reply({
          content: "A moderação não está configurada neste bot.",
          flags: MessageFlags.Ephemeral,
        });
        return;
      }
      const guild = it.guild;
      await handleMod(adaptMod(it), {
        cfg: moderation,
        state,
        timeout: async (userId, ms, reason) => {
          const member = await guild?.members.fetch(userId);
          if (!member) throw new Error("membro não encontrado");
          await member.timeout(ms, reason);
        },
        ban: async (userId, reason, deleteSeconds) => {
          if (!guild) throw new Error("fora de um servidor");
          await guild.members.ban(userId, { reason, deleteMessageSeconds: deleteSeconds });
        },
        log: modLog,
        now: () => Date.now(),
      });
    }
  } catch (e) {
    const err = e as Error;
    console.error(`falha ao responder interação: ${err.name}: ${err.message}`);
  }
});

async function shutdown() {
  for (const stop of stops) stop();
  await mcp.close();
  state.close();
  await client.destroy();
  process.exit(0);
}
process.on("SIGINT", () => void shutdown());
process.on("SIGTERM", () => void shutdown());

await client.login(env.token);
