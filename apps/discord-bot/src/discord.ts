/** Ponte entre as interações do discord.js e o formato testável usado pelos comandos. */
import {
  MessageFlags,
  type AutocompleteInteraction,
  type ChatInputCommandInteraction,
} from "discord.js";

import type { ChannelRef } from "./access.ts";
import type { NexusInteraction, ReplyPayload } from "./commands/nexus.ts";

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
    content: p.content,
    allowedMentions: p.allowedMentions,
    ...(p.files ? { files: p.files.map((f) => ({ attachment: f.attachment, name: f.name })) } : {}),
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
