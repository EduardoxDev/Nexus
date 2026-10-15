/** Boas-vindas a quem entra no servidor, com o embed da marca. */
import { escapeMarkdown } from "discord.js";

import { brandHeader, logoFile, type BotMessage } from "../format.ts";
import type { WelcomeConfig } from "./config.ts";

export interface NewMember {
  id: string;
  displayName: string;
  isBot: boolean;
  guildName: string;
  memberCount: number;
}

/** Mensagem de boas-vindas; menciona só a pessoa que entrou. `null` para bots. */
export function formatWelcome(
  cfg: WelcomeConfig,
  m: NewMember,
): (BotMessage & { content: string; mentionUserId: string }) | null {
  if (m.isBot) return null;
  const name = escapeMarkdown(m.displayName).slice(0, 100);
  const server = escapeMarkdown(m.guildName);
  const description = cfg.message
    .replaceAll("{usuario}", `<@${m.id}>`)
    .replaceAll("{servidor}", server)
    .slice(0, 4_096);
  return {
    content: `<@${m.id}>`,
    mentionUserId: m.id,
    embeds: [
      {
        ...brandHeader("Boas-vindas"),
        title: `👋 Bem-vindo(a), ${name}!`.slice(0, 256),
        description,
        footer: { text: `Membro nº ${m.memberCount}` },
      },
    ],
    files: [logoFile()],
  };
}
