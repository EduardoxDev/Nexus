/**
 * `/painel cargos|tickets` publica os painéis com botões no canal atual (só para quem pode
 * gerenciar o servidor). `/ticket fechar` encerra o ticket da thread onde é usado.
 */
import { InteractionContextType, PermissionFlagsBits, SlashCommandBuilder } from "discord.js";

import type { ManagementConfig } from "../management/config.ts";
import {
  rolePanel,
  rolePanelProblems,
  type PanelMessage,
  type RoleInfo,
} from "../management/roles.ts";
import { ticketPanel } from "../management/tickets.ts";

export const panelCommand = new SlashCommandBuilder()
  .setName("painel")
  .setDescription("Publica um painel com botões neste canal (administração)")
  .setDefaultMemberPermissions(PermissionFlagsBits.ManageGuild)
  .setContexts(InteractionContextType.Guild)
  .addSubcommand((s) => s.setName("cargos").setDescription("Botões para pegar e tirar cargos"))
  .addSubcommand((s) => s.setName("tickets").setDescription("Botão para abrir ticket"));

export const ticketCommand = new SlashCommandBuilder()
  .setName("ticket")
  .setDescription("Tickets de suporte")
  .setContexts(InteractionContextType.Guild)
  .addSubcommand((s) => s.setName("fechar").setDescription("Fecha o ticket desta thread"));

export interface PanelInteraction {
  canManageGuild: boolean;
  subcommand: string;
  /** Publica no canal da interação. */
  post(panel: PanelMessage): Promise<void>;
  /** Resposta que só quem usou vê. */
  reply(text: string): Promise<void>;
}

export async function handlePanel(
  i: PanelInteraction,
  cfg: ManagementConfig | null,
  roleInfo: (roleId: string) => RoleInfo,
): Promise<void> {
  if (!i.canManageGuild) {
    await i.reply("Só quem pode gerenciar o servidor publica painéis.");
    return;
  }
  if (i.subcommand === "cargos") {
    const roles = cfg?.roles ?? [];
    if (roles.length === 0) {
      await i.reply("Nenhum cargo configurado em `management.roles`.");
      return;
    }
    const problems = rolePanelProblems(roles, roleInfo);
    if (problems.length > 0) {
      await i.reply(`Painel não publicado:\n${problems.map((p) => `• ${p}`).join("\n")}`);
      return;
    }
    await i.post(rolePanel(roles));
    await i.reply("Painel de cargos publicado.");
    return;
  }
  if (i.subcommand === "tickets") {
    if (!cfg?.tickets) {
      await i.reply("Tickets não configurados em `management.tickets`.");
      return;
    }
    await i.post(ticketPanel());
    await i.reply("Painel de tickets publicado.");
    return;
  }
  await i.reply("Comando desconhecido.");
}
