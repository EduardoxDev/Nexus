/**
 * `/nexus`: as funções da NEXUS pelo Discord. Toda consulta passa pela regra de acesso antes de
 * chegar à NEXUS; um projeto da equipe pedido fora de canal da equipe é recusado sem revelar nada.
 */
import { SlashCommandBuilder, type APIEmbed } from "discord.js";

import {
  findProject,
  isTeamChannel,
  sameProject,
  visibleProjects,
  type ChannelRef,
} from "../access.ts";
import type { BotConfig } from "../config.ts";
import {
  formatError,
  formatPack,
  formatSearch,
  formatStatus,
  formatVersion,
  type BotMessage,
} from "../format.ts";
import { McpToolError, McpUnavailableError } from "../mcp/client.ts";
import type { PackResult, ProjectStatus, SearchResult } from "../nexus/types.ts";
import type { Release } from "../releases.ts";

/** O que o comando precisa da NEXUS (o `McpClient` satisfaz). */
export interface Tools {
  callTool<T>(name: string, args: Record<string, unknown>): Promise<T>;
  /** Versão da NEXUS em uso (do handshake MCP). */
  serverVersion?(): Promise<string | null>;
}

export interface ReplyPayload {
  content?: string;
  embeds?: APIEmbed[];
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

/** Toda resposta: embeds + anexos, sem menções. */
const payload = (m: BotMessage): ReplyPayload => ({ ...m, allowedMentions: NO_MENTIONS });
const MSG = {
  refused: "Este projeto não pode ser consultado neste canal.",
  unavailable: "A NEXUS não respondeu agora. Tente de novo em instantes.",
  failed: "Não foi possível concluir o pedido.",
};
const MAX_TEXT = 300;
/**
 * Prazo do comando inteiro. O cliente MCP tem prazo por requisição; uma `pergunta` faz até duas
 * buscas (e o servidor pode estar reiniciando), então a soma poderia chegar a ~90 s.
 */
export const COMMAND_DEADLINE_MS = 30_000;

/** Rejeita com `McpUnavailableError` se `work` não terminar no prazo. */
async function withDeadline<T>(work: Promise<T>, ms: number): Promise<T> {
  let timer: NodeJS.Timeout | undefined;
  const timeout = new Promise<never>((_, reject) => {
    timer = setTimeout(
      () => reject(new McpUnavailableError(`o comando passou do prazo de ${ms} ms`)),
      ms,
    );
  });
  try {
    return await Promise.race([work, timeout]);
  } finally {
    clearTimeout(timer);
  }
}

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
  .addSubcommand((s) => s.setName("status").setDescription("Estado do índice dos projetos"))
  .addSubcommand((s) =>
    s.setName("versao").setDescription("Versão da NEXUS em uso e a última publicada"),
  );

/**
 * Projeto pedido (com a grafia da configuração, que é o que vai para a NEXUS), ou o padrão do
 * canal: o da equipe em canais da equipe, senão o público. `null` = não pode neste canal.
 */
function resolveProject(i: NexusInteraction, cfg: BotConfig): string | null {
  const asked = i.option("projeto")?.trim();
  if (asked) return findProject(cfg, i.channel, asked);
  if (isTeamChannel(cfg, i.channel) && cfg.teamProjects[0]) return cfg.teamProjects[0];
  return cfg.publicProject;
}

export async function handleNexus(
  i: NexusInteraction,
  deps: {
    tools: Tools;
    cfg: BotConfig;
    log?: (line: string) => void;
    deadlineMs?: number;
    /** Última release publicada (GitHub), para `/nexus versao`. */
    latestRelease?: () => Promise<Release | null>;
  },
): Promise<void> {
  const { tools, cfg, log = console.error, deadlineMs = COMMAND_DEADLINE_MS } = deps;
  const sub = i.subcommand;
  const project = sub === "pergunta" ? cfg.publicProject : resolveProject(i, cfg);
  // `status` e `versao` não consultam um projeto (status filtra pelo canal).
  if (sub !== "status" && sub !== "versao" && project === null) {
    await i.reply({ ...payload(formatError(MSG.refused)), ephemeral: true });
    return;
  }

  await i.deferReply();
  let reply: BotMessage;
  try {
    reply = await withDeadline(
      answer(i, sub, project ?? cfg.publicProject, tools, cfg, deps.latestRelease),
      deadlineMs,
    );
  } catch (e) {
    // Motivo técnico no terminal (sem o texto digitado nem conteúdo do repositório).
    const err = e as Error;
    log(`/nexus ${sub} falhou: ${err.name}: ${err.message}`);
    const message =
      e instanceof McpUnavailableError
        ? MSG.unavailable
        : e instanceof McpToolError
          ? `${MSG.failed} ${e.message}`
          : MSG.failed;
    reply = formatError(message);
  }
  // Só aqui a resposta sai: um resultado que chegue depois do prazo não sobrescreve o erro.
  await i.editReply(payload(reply));
}

/** Calcula a resposta de um subcomando (sem enviar nada). */
async function answer(
  i: NexusInteraction,
  sub: string,
  project: string,
  tools: Tools,
  cfg: BotConfig,
  latestRelease?: () => Promise<Release | null>,
): Promise<BotMessage> {
  if (sub === "versao") {
    const installed = tools.serverVersion ? await tools.serverVersion() : null;
    // Sem GitHub (rede fora, limite da API), ainda mostra a versão em uso.
    const latest = latestRelease ? await latestRelease().catch(() => null) : null;
    return formatVersion(installed, latest);
  }
  if (sub === "pergunta" || sub === "buscar") {
    const query = i.option("texto") ?? "";
    const search = (extra: Record<string, unknown> = {}) =>
      tools.callTool<SearchResult>("search_project", { project, query, limit: 3, ...extra });
    // Dúvidas da comunidade: documentação primeiro; código só se a documentação não responder.
    let r = sub === "pergunta" ? await search({ kinds: ["doc"] }) : await search();
    if (sub === "pergunta" && r.hits.length === 0) r = await search();
    return formatSearch(query, r);
  }
  if (sub === "pack") {
    const task = i.option("tarefa") ?? "";
    const p = await tools.callTool<PackResult>("build_context_pack", {
      project,
      task,
      format: "markdown",
    });
    return formatPack(task, p);
  }
  const r = await tools.callTool<{ projects: ProjectStatus[] }>("list_projects", {});
  const visible = visibleProjects(cfg, i.channel);
  return formatStatus(r.projects.filter((p) => visible.some((v) => sameProject(v, p.name))));
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
