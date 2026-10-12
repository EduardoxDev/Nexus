/**
 * `/nexus`: as funções da NEXUS pelo Discord. Toda consulta passa pela regra de acesso antes de
 * chegar à NEXUS; um projeto da equipe pedido fora de canal da equipe é recusado sem revelar nada.
 */
import { SlashCommandBuilder } from "discord.js";

import { canQuery, isTeamChannel, visibleProjects, type ChannelRef } from "../access.ts";
import type { BotConfig } from "../config.ts";
import { formatPack, formatSearch, formatStatus } from "../format.ts";
import { McpToolError, McpUnavailableError } from "../mcp/client.ts";
import type { PackResult, ProjectStatus, SearchResult } from "../nexus/types.ts";

/** O que o comando precisa da NEXUS (o `McpClient` satisfaz). */
export interface Tools {
  callTool<T>(name: string, args: Record<string, unknown>): Promise<T>;
}

export interface ReplyPayload {
  content: string;
  files?: { name: string; attachment: Buffer }[];
  /** Sempre vazio: nada vindo do repositório ou do usuário pode notificar alguém. */
  allowedMentions: { parse: [] };
  ephemeral?: boolean;
}

/** O que o comando usa de uma interação do Discord (adaptado em `index.ts`). */
export interface NexusInteraction {
  channel: ChannelRef;
  subcommand: string;
  option(name: string): string | null;
  deferReply(): Promise<unknown>;
  editReply(payload: ReplyPayload): Promise<unknown>;
  reply(payload: ReplyPayload): Promise<unknown>;
}

const NO_MENTIONS = { parse: [] } as { parse: [] };
const MSG = {
  refused: "Este projeto não pode ser consultado neste canal.",
  unavailable: "A NEXUS não respondeu agora. Tente de novo em instantes.",
  failed: "Não foi possível concluir o pedido.",
};
const MAX_TEXT = 300;

export const nexusCommand = new SlashCommandBuilder()
  .setName("nexus")
  .setDescription("Busca e contexto dos projetos com a NEXUS")
  .addSubcommand((s) =>
    s
      .setName("pergunta")
      .setDescription("Pergunte sobre a NEXUS; a resposta cita a documentação e o código")
      .addStringOption((o) =>
        o.setName("texto").setDescription("Sua pergunta").setRequired(true).setMaxLength(MAX_TEXT),
      ),
  )
  .addSubcommand((s) =>
    s
      .setName("buscar")
      .setDescription("Busca em código e documentação, com fontes")
      .addStringOption((o) =>
        o
          .setName("texto")
          .setDescription("O que procurar")
          .setRequired(true)
          .setMaxLength(MAX_TEXT),
      )
      .addStringOption((o) =>
        o.setName("projeto").setDescription("Projeto (padrão: o do canal)").setAutocomplete(true),
      ),
  )
  .addSubcommand((s) =>
    s
      .setName("pack")
      .setDescription("Monta um Context Pack para uma tarefa (anexo .md)")
      .addStringOption((o) =>
        o
          .setName("tarefa")
          .setDescription("Descreva a tarefa")
          .setRequired(true)
          .setMaxLength(MAX_TEXT),
      )
      .addStringOption((o) =>
        o.setName("projeto").setDescription("Projeto (padrão: o do canal)").setAutocomplete(true),
      ),
  )
  .addSubcommand((s) => s.setName("status").setDescription("Estado do índice dos projetos"));

/** Projeto pedido, ou o padrão do canal: o da equipe em canais da equipe, senão o público. */
function resolveProject(i: NexusInteraction, cfg: BotConfig): string {
  const asked = i.option("projeto")?.trim();
  if (asked) return asked;
  if (isTeamChannel(cfg, i.channel) && cfg.teamProjects[0]) return cfg.teamProjects[0];
  return cfg.publicProject;
}

export async function handleNexus(
  i: NexusInteraction,
  deps: { tools: Tools; cfg: BotConfig },
): Promise<void> {
  const { tools, cfg } = deps;
  const sub = i.subcommand;
  const project = sub === "pergunta" ? cfg.publicProject : resolveProject(i, cfg);
  if (sub !== "status" && !canQuery(cfg, i.channel, project)) {
    await i.reply({ content: MSG.refused, allowedMentions: NO_MENTIONS, ephemeral: true });
    return;
  }

  await i.deferReply();
  try {
    if (sub === "pergunta" || sub === "buscar") {
      const query = i.option("texto") ?? "";
      const search = (extra: Record<string, unknown> = {}) =>
        tools.callTool<SearchResult>("search_project", { project, query, limit: 3, ...extra });
      // Dúvidas da comunidade: documentação primeiro; código só se a documentação não responder.
      let r = sub === "pergunta" ? await search({ kinds: ["doc"] }) : await search();
      if (sub === "pergunta" && r.hits.length === 0) r = await search();
      await i.editReply({ content: formatSearch(query, r), allowedMentions: NO_MENTIONS });
    } else if (sub === "pack") {
      const task = i.option("tarefa") ?? "";
      const p = await tools.callTool<PackResult>("build_context_pack", {
        project,
        task,
        format: "markdown",
      });
      const out = formatPack(task, p);
      await i.editReply({
        content: out.content,
        files: [{ name: out.file.name, attachment: out.file.data }],
        allowedMentions: NO_MENTIONS,
      });
    } else {
      const r = await tools.callTool<{ projects: ProjectStatus[] }>("list_projects", {});
      const visible = visibleProjects(cfg, i.channel).map((p) => p.toLowerCase());
      const shown = r.projects.filter((p) => visible.includes(p.name.toLowerCase()));
      await i.editReply({ content: formatStatus(shown), allowedMentions: NO_MENTIONS });
    }
  } catch (e) {
    const content =
      e instanceof McpUnavailableError
        ? MSG.unavailable
        : e instanceof McpToolError
          ? `${MSG.failed} ${e.message}`.slice(0, 1900)
          : MSG.failed;
    await i.editReply({ content, allowedMentions: NO_MENTIONS });
  }
}

/** Sugestões da opção `projeto`: só projetos visíveis neste canal. */
export function autocompleteProjects(
  focused: string,
  ch: ChannelRef,
  cfg: BotConfig,
): { name: string; value: string }[] {
  const q = focused.trim().toLowerCase();
  return visibleProjects(cfg, ch)
    .filter((p) => p.toLowerCase().includes(q))
    .slice(0, 25)
    .map((p) => ({ name: p, value: p }));
}
