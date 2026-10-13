/** Ponte entre as interações do discord.js e o formato testável usado pelos comandos. */
import {
  Client,
  Events,
  GatewayIntentBits,
  MessageFlags,
  Partials,
  type AutocompleteInteraction,
  type ChatInputCommandInteraction,
} from "discord.js";

import type { ChannelRef } from "./access.ts";
import type { BotMessage } from "./format.ts";
import type { NexusInteraction, ReplyPayload } from "./commands/nexus.ts";

/**
 * Cliente do Discord do bot. `Partials.Channel` faz canais fora do cache (ex.: threads privadas)
 * chegarem como parciais com o canal pai; erros do gateway são registrados em vez de derrubar o
 * processo (sem listener, o Node trata o evento `error` como exceção).
 */
export function createClient(): Client {
  const client = new Client({ intents: [GatewayIntentBits.Guilds], partials: [Partials.Channel] });
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
