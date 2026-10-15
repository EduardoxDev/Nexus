/** Ponte entre as interações do discord.js e o formato testável usado pelos comandos. */
import {
  Client,
  Events,
  GatewayIntentBits,
  MessageFlags,
  Partials,
  PermissionFlagsBits,
  type AutocompleteInteraction,
  type ChatInputCommandInteraction,
  type Guild,
  type GuildMember,
  type Message,
  type SendableChannels,
} from "discord.js";

import type { ChannelRef } from "./access.ts";
import type { BotMessage } from "./format.ts";
import type { ModInteraction } from "./commands/mod.ts";
import type { NexusInteraction, ReplyPayload } from "./commands/nexus.ts";
import type { PanelMessage, RoleInfo } from "./management/roles.ts";
import type { MessageInfo } from "./moderation/automod.ts";

export interface IntentOptions {
  automod: boolean;
  /** Boas-vindas (entrada de membros). */
  members?: boolean;
}

/**
 * Intents pedidas ao Discord. Mensagens (e a intent privilegiada Message Content) só com o
 * automod ligado, e Server Members só com as boas-vindas: pedir uma intent privilegiada não
 * habilitada no Developer Portal faz o login falhar.
 */
export function intentsFor(opts: IntentOptions): GatewayIntentBits[] {
  const intents = [GatewayIntentBits.Guilds];
  if (opts.automod) intents.push(GatewayIntentBits.GuildMessages, GatewayIntentBits.MessageContent);
  if (opts.members) intents.push(GatewayIntentBits.GuildMembers);
  return intents;
}

/**
 * Cliente do Discord do bot. `Partials.Channel` faz canais fora do cache (ex.: threads privadas)
 * chegarem como parciais com o canal pai; erros do gateway são registrados em vez de derrubar o
 * processo (sem listener, o Node trata o evento `error` como exceção).
 */
export function createClient(opts: IntentOptions = { automod: false }): Client {
  const client = new Client({ intents: intentsFor(opts), partials: [Partials.Channel] });
  client.on(Events.Error, (e) => console.error(`erro do Discord: ${e.name}`));
  return client;
}

/**
 * Canal da interação. Só threads herdam o canal pai: o "pai" de um canal comum é a categoria,
 * e uma categoria não deve liberar acesso por acidente.
 */
export function channelRef(i: ChatInputCommandInteraction | AutocompleteInteraction): ChannelRef {
  const ch = i.channel;
  const parentId = ch?.isThread() ? ch.parentId : null;
  return { id: i.channelId ?? "", parentId: parentId ?? null };
}

function toDiscord(p: ReplyPayload) {
  return {
    ...(p.content !== undefined ? { content: p.content } : {}),
    ...(p.embeds ? { embeds: p.embeds } : {}),
    allowedMentions: p.allowedMentions,
    ...(p.files ? { files: p.files.map((f) => ({ attachment: f.attachment, name: f.name })) } : {}),
  };
}

/** Publica uma mensagem num canal (anúncios e avisos). Nunca menciona ninguém. */
export async function sendTo(client: Client, channelId: string, m: BotMessage): Promise<void> {
  const channel = await client.channels.fetch(channelId);
  if (!channel?.isSendable()) {
    throw new Error(`o canal ${channelId} não existe ou o bot não pode escrever nele`);
  }
  await channel.send(toDiscord({ ...m, allowedMentions: { parse: [] } }));
}

/** Permissões que fazem de um cargo um cargo de equipe: o bot nunca o dá por botão. */
const STAFF_PERMISSIONS = [
  PermissionFlagsBits.Administrator,
  PermissionFlagsBits.ManageGuild,
  PermissionFlagsBits.ManageRoles,
  PermissionFlagsBits.ManageChannels,
  PermissionFlagsBits.ManageMessages,
  PermissionFlagsBits.ManageThreads,
  PermissionFlagsBits.ManageWebhooks,
  PermissionFlagsBits.ManageNicknames,
  PermissionFlagsBits.KickMembers,
  PermissionFlagsBits.BanMembers,
  PermissionFlagsBits.ModerateMembers,
  PermissionFlagsBits.MentionEveryone,
];

/** Como o Discord vê um cargo agora (do cache do servidor). */
export function roleInfoOf(guild: Guild, roleId: string): RoleInfo {
  const role = guild.roles.cache.get(roleId);
  if (!role) return { exists: false, dangerous: false, editable: false };
  return {
    exists: true,
    dangerous: role.managed || role.id === guild.id || role.permissions.any(STAFF_PERMISSIONS),
    editable: role.editable,
  };
}

/** Publica um painel com botões. Nunca menciona ninguém. */
export async function postPanel(channel: SendableChannels, p: PanelMessage): Promise<void> {
  await channel.send({
    embeds: p.embeds,
    files: p.files.map((f) => ({ attachment: f.attachment, name: f.name })),
    components: p.components,
    allowedMentions: { parse: [] },
  });
}

/** Resumo da mensagem para o automod (sem guardar o conteúdo em lugar nenhum). */
export function messageInfo(m: Message): MessageInfo {
  const ch = m.channel;
  return {
    authorId: m.author.id,
    authorIsBot: m.author.bot || m.system || m.webhookId !== null,
    roleIds: m.member ? [...m.member.roles.cache.keys()] : [],
    channelId: m.channelId,
    parentId: ch.isThread() ? ch.parentId : null,
    content: m.content,
    mentionCount: m.mentions.users.size + m.mentions.roles.size + (m.mentions.everyone ? 1 : 0),
    at: m.createdTimestamp,
  };
}

/** Interação de `/mod` no formato testável. */
export function adaptMod(i: ChatInputCommandInteraction): ModInteraction {
  const actor = i.member as GuildMember | null;
  const user = i.options.getUser("usuario");
  const member = i.options.getMember("usuario") as GuildMember | null;
  const guild = i.guild;
  const outranks = (): boolean => {
    if (member === null) return true;
    if (!guild || !actor?.roles?.highest || !member.roles?.highest) return false;
    if (member.id === guild.ownerId) return false;
    if (actor.id === guild.ownerId) return true;
    return actor.roles.highest.comparePositionTo(member.roles.highest) > 0;
  };
  return {
    actorId: i.user.id,
    actorRoleIds: actor?.roles?.cache ? [...actor.roles.cache.keys()] : [],
    canModerate: i.memberPermissions?.has(PermissionFlagsBits.ModerateMembers) ?? false,
    canBan: i.memberPermissions?.has(PermissionFlagsBits.BanMembers) ?? false,
    subcommand: i.options.getSubcommand(),
    target: user
      ? {
          id: user.id,
          isBot: user.bot,
          inGuild: member !== null,
          moderatable: member?.moderatable ?? false,
          bannable: member?.bannable ?? true,
          belowActor: outranks(),
        }
      : null,
    string: (name) => i.options.getString(name),
    integer: (name) => i.options.getInteger(name),
    defer: () => i.deferReply({ flags: MessageFlags.Ephemeral }),
    reply: (p) =>
      i.deferred || i.replied
        ? i.editReply(toDiscord(p))
        : i.reply({ ...toDiscord(p), ...(p.ephemeral ? { flags: MessageFlags.Ephemeral } : {}) }),
  };
}

export function adapt(i: ChatInputCommandInteraction): NexusInteraction {
  return {
    channel: channelRef(i),
    subcommand: i.options.getSubcommand(),
    option: (name) => i.options.getString(name),
    deferReply: () => i.deferReply(),
    editReply: (p) => i.editReply(toDiscord(p)),
    reply: (p) =>
      i.reply({ ...toDiscord(p), ...(p.ephemeral ? { flags: MessageFlags.Ephemeral } : {}) }),
  };
}
