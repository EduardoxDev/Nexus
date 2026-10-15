/**
 * Inicialização do bot: ambiente, regras de acesso, cliente MCP da NEXUS e Discord.
 * Logs registram só eventos — nunca o token nem conteúdo de mensagens ou do repositório.
 */
import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";

import {
  ChannelType,
  Events,
  MessageFlags,
  PermissionFlagsBits,
  ThreadAutoArchiveDuration,
  type ButtonInteraction,
  type ChatInputCommandInteraction,
  type GuildMember,
} from "discord.js";

import { collectNotices } from "./alerts.ts";
import { handleMod } from "./commands/mod.ts";
import { handlePanel } from "./commands/panel.ts";
import { autocompleteProjects, handleNexus } from "./commands/nexus.ts";
import { loadConfig, loadEnv } from "./config.ts";
import {
  adapt,
  adaptMod,
  channelRef,
  createClient,
  messageInfo,
  postPanel,
  roleInfoOf,
  sendTo,
} from "./discord.ts";
import { formatNotices, formatRelease, type BotMessage } from "./format.ts";
import { every } from "./poll.ts";
import { latestRelease, shouldAnnounce } from "./releases.ts";
import { BotState } from "./state.ts";
import { AlreadyRunningError, acquireInstanceLock } from "./instance.ts";
import { McpClient } from "./mcp/client.ts";
import { ROLE_PREFIX, toggleRole } from "./management/roles.ts";
import { closeTicket, openTicket, TICKET_CLOSE, TICKET_OPEN } from "./management/tickets.ts";
import { formatWelcome } from "./management/welcome.ts";
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
const management = cfg.management;
const client = createClient({
  automod: moderation?.automod ?? false,
  members: management?.welcome != null,
});
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
/** Registro dos tickets, se houver canal. */
const ticketLog = async (m: BotMessage) => {
  const ch = management?.tickets?.logChannelId;
  if (!ch) return;
  try {
    await sendTo(client, ch, m);
  } catch (e) {
    console.error(`registro de tickets falhou: ${(e as Error).name}`);
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

client.on(Events.GuildMemberAdd, async (member) => {
  const welcome = management?.welcome;
  if (!welcome || member.guild.id !== env.guildId) return;
  try {
    const m = formatWelcome(welcome, {
      id: member.id,
      displayName: member.displayName,
      isBot: member.user.bot,
      guildName: member.guild.name,
      memberCount: member.guild.memberCount,
    });
    if (!m) return;
    const channel = await client.channels.fetch(welcome.channelId);
    if (!channel?.isSendable()) throw new Error("canal de boas-vindas indisponível");
    await channel.send({
      content: m.content,
      embeds: m.embeds,
      files: m.files.map((f) => ({ attachment: f.attachment, name: f.name })),
      allowedMentions: { users: [m.mentionUserId] },
    });
  } catch (e) {
    console.error(`boas-vindas falhou: ${(e as Error).name}`);
  }
});

const ephemeral = (text: string) => ({
  content: text,
  allowedMentions: { parse: [] as [] },
  flags: MessageFlags.Ephemeral as const,
});

async function onTicketClose(it: ButtonInteraction | ChatInputCommandInteraction) {
  const tickets = management?.tickets;
  if (!tickets) {
    await it.reply(ephemeral("Tickets não configurados."));
    return;
  }
  const member = it.member as GuildMember | null;
  const thread = it.channel;
  await closeTicket(
    tickets,
    state,
    it.channelId ?? "",
    {
      id: it.user.id,
      roleIds: member?.roles?.cache ? [...member.roles.cache.keys()] : [],
      canManageThreads: it.memberPermissions?.has(PermissionFlagsBits.ManageThreads) ?? false,
    },
    {
      announce: async (text) => {
        await it.reply({ content: text, allowedMentions: { parse: [] } });
      },
      refuse: async (text) => {
        await it.reply(ephemeral(text));
      },
      archive: async () => {
        if (!thread?.isThread()) return;
        await thread.setLocked(true);
        await thread.setArchived(true);
      },
      log: ticketLog,
    },
    Date.now(),
  );
}

async function onButton(it: ButtonInteraction) {
  if (it.customId.startsWith(ROLE_PREFIX)) {
    const member = it.member as GuildMember | null;
    const guild = it.guild;
    if (!member?.roles?.cache || !guild) return;
    const text = await toggleRole(it.customId, management?.roles ?? [], {
      memberRoleIds: [...member.roles.cache.keys()],
      info: (roleId) => roleInfoOf(guild, roleId),
      add: async (roleId) => {
        await member.roles.add(roleId, "cargo por botão");
      },
      remove: async (roleId) => {
        await member.roles.remove(roleId, "cargo por botão");
      },
    });
    await it.reply(ephemeral(text));
  } else if (it.customId === TICKET_OPEN) {
    const tickets = management?.tickets;
    if (!tickets) {
      await it.reply(ephemeral("Tickets não configurados."));
      return;
    }
    await it.deferReply({ flags: MessageFlags.Ephemeral });
    const text = await openTicket(
      tickets,
      state,
      { id: it.user.id, username: it.user.username },
      {
        createThread: async (name) => {
          const parent = await client.channels.fetch(tickets.channelId);
          if (parent?.type !== ChannelType.GuildText) {
            throw new Error("o canal de tickets precisa ser um canal de texto");
          }
          const thread = await parent.threads.create({
            name,
            type: ChannelType.PrivateThread,
            invitable: false,
            autoArchiveDuration: ThreadAutoArchiveDuration.OneWeek,
            reason: "ticket de suporte",
          });
          return thread.id;
        },
        addMember: async (threadId, userId) => {
          const thread = await client.channels.fetch(threadId);
          if (thread?.isThread()) await thread.members.add(userId);
        },
        postIntro: async (threadId, m) => {
          const thread = await client.channels.fetch(threadId);
          if (!thread?.isSendable()) throw new Error("thread do ticket indisponível");
          // Mencionar os cargos da equipe numa thread privada traz a equipe para ela.
          await thread.send({
            content: m.content,
            embeds: m.embeds,
            files: m.files.map((f) => ({ attachment: f.attachment, name: f.name })),
            components: m.components,
            allowedMentions: { users: [it.user.id], roles: tickets.staffRoleIds },
          });
        },
        isOpen: async (threadId) => {
          try {
            const thread = await client.channels.fetch(threadId);
            return thread?.isThread() === true && !thread.archived;
          } catch {
            return false;
          }
        },
        log: ticketLog,
      },
      Date.now(),
    );
    await it.editReply({ content: text, allowedMentions: { parse: [] } });
  } else if (it.customId === TICKET_CLOSE) {
    await onTicketClose(it);
  }
}

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
    } else if (it.isChatInputCommand() && it.commandName === "painel") {
      const guild = it.guild;
      const channel = it.channel;
      if (!guild || !channel?.isSendable()) return;
      await handlePanel(
        {
          canManageGuild: it.memberPermissions?.has(PermissionFlagsBits.ManageGuild) ?? false,
          subcommand: it.options.getSubcommand(),
          post: (p) => postPanel(channel, p),
          reply: async (text) => {
            await it.reply(ephemeral(text));
          },
        },
        management,
        (roleId) => roleInfoOf(guild, roleId),
      );
    } else if (it.isChatInputCommand() && it.commandName === "ticket") {
      await onTicketClose(it);
    } else if (it.isButton()) {
      await onButton(it);
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
