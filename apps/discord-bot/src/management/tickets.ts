/**
 * Tickets: o botão "Abrir ticket" cria uma thread privada com a pessoa e a equipe; o botão
 * "Fechar" (ou `/ticket fechar`) encerra, tranca e arquiva. Uma pessoa tem no máximo um ticket
 * aberto. O Discord fica atrás de `TicketPorts` (os testes usam portas falsas).
 */
import { ButtonStyle, ComponentType } from "discord.js";

import { brandHeader, logoFile, type BotMessage } from "../format.ts";
import type { BotState } from "../state.ts";
import type { TicketsConfig } from "./config.ts";
import { rows, type PanelMessage } from "./roles.ts";

export const TICKET_OPEN = "nexus:ticket:open";
export const TICKET_CLOSE = "nexus:ticket:close";

export function ticketPanel(): PanelMessage {
  return {
    embeds: [
      {
        ...brandHeader("Suporte"),
        title: "🎫 Precisa de ajuda?",
        description:
          "Clique em **Abrir ticket** para falar com a equipe numa conversa privada: só você e a equipe veem.",
      },
    ],
    files: [logoFile()],
    components: rows([
      {
        type: ComponentType.Button,
        style: ButtonStyle.Primary,
        custom_id: TICKET_OPEN,
        label: "Abrir ticket",
        emoji: { name: "🎫" },
      },
    ]),
  };
}

/** Primeira mensagem da thread: chama a pessoa e a equipe e traz o botão de fechar. */
export function ticketIntro(
  cfg: TicketsConfig,
  ownerId: string,
): PanelMessage & { content: string } {
  const staff = cfg.staffRoleIds.map((r) => `<@&${r}>`).join(" ");
  return {
    content: `<@${ownerId}> ${staff}`.trim(),
    embeds: [
      {
        ...brandHeader("Suporte"),
        title: "🎫 Ticket aberto",
        description:
          "Conte o que precisa; a equipe responde por aqui. Quando terminar, clique em **Fechar ticket**.",
      },
    ],
    files: [logoFile()],
    components: rows([
      {
        type: ComponentType.Button,
        style: ButtonStyle.Danger,
        custom_id: TICKET_CLOSE,
        label: "Fechar ticket",
      },
    ]),
  };
}

/** Nome da thread: só letras, números e hífen, como num nome de canal. */
export function ticketName(username: string): string {
  const slug = username
    .normalize("NFKD")
    .replace(/[̀-ͯ]/gu, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/gu, "-")
    .replace(/^-+|-+$/gu, "")
    .slice(0, 40);
  return `ticket-${slug || "membro"}`;
}

function logEntry(title: string, lines: string[]): BotMessage {
  return {
    embeds: [{ ...brandHeader("Tickets"), title, description: lines.join("\n") }],
    files: [logoFile()],
  };
}

export interface TicketPorts {
  /** Cria a thread privada (sem convites de terceiros) e devolve o ID. */
  createThread(name: string): Promise<string>;
  addMember(threadId: string, userId: string): Promise<void>;
  /** Publica a abertura, mencionando só a pessoa e os cargos da equipe. */
  postIntro(threadId: string, m: PanelMessage & { content: string }): Promise<void>;
  /** Apaga a thread de uma abertura que não terminou. */
  deleteThread(threadId: string): Promise<void>;
  /** A thread ainda existe e está aberta (um ticket apagado à mão não trava a pessoa). */
  isOpen(threadId: string): Promise<boolean>;
  log(m: BotMessage): Promise<void>;
}

/** Cliques simultâneos da mesma pessoa não abrem dois tickets. */
const opening = new Set<string>();

export async function openTicket(
  cfg: TicketsConfig,
  state: BotState,
  user: { id: string; username: string },
  p: TicketPorts,
  now: number,
): Promise<string> {
  if (opening.has(user.id)) return "Seu ticket já está sendo aberto.";
  opening.add(user.id);
  try {
    const current = state.openTicketOf(user.id);
    if (current) {
      if (await p.isOpen(current)) return `Você já tem um ticket aberto: <#${current}>.`;
      state.closeTicket(current, "sistema", now);
    }
    const threadId = await p.createThread(ticketName(user.username));
    try {
      await p.addMember(threadId, user.id);
      await p.postIntro(threadId, ticketIntro(cfg, user.id));
    } catch (e) {
      // Abertura pela metade: some com a thread e não registra nada (a pessoa pode tentar de novo).
      await p
        .deleteThread(threadId)
        .catch((err: Error) => console.error(`apagar ticket incompleto falhou: ${err.name}`));
      throw e;
    }
    // Só registra o ticket depois que ele está pronto.
    state.recordTicket(threadId, user.id, now);
    await p.log(logEntry("Ticket aberto", [`<#${threadId}>`, `Por <@${user.id}>`]));
    return `Ticket aberto: <#${threadId}>.`;
  } catch (e) {
    console.error(`abrir ticket falhou: ${(e as Error).name}`);
    return "Não consegui abrir o ticket agora. Avise a equipe.";
  } finally {
    opening.delete(user.id);
  }
}

export interface CloseActor {
  id: string;
  roleIds: string[];
  /** Permissão **Gerenciar threads** no canal. */
  canManageThreads: boolean;
}

export interface ClosePorts {
  /** Resposta visível na thread (antes de arquivar, senão não dá para responder). */
  announce(text: string): Promise<void>;
  refuse(text: string): Promise<void>;
  /** Aviso, só para quem pediu, de que o fechamento não terminou. */
  failed(text: string): Promise<void>;
  archive(threadId: string): Promise<void>;
  log(m: BotMessage): Promise<void>;
}

/** Dois pedidos de fechar ao mesmo tempo viram um só. */
const closing = new Set<string>();

export async function closeTicket(
  cfg: TicketsConfig,
  state: BotState,
  threadId: string,
  actor: CloseActor,
  p: ClosePorts,
  now: number,
): Promise<void> {
  const t = state.ticket(threadId);
  if (!t || t.closedAt !== null) {
    await p.refuse("Aqui não é um ticket aberto.");
    return;
  }
  const staff = actor.canManageThreads || actor.roleIds.some((r) => cfg.staffRoleIds.includes(r));
  if (actor.id !== t.ownerId && !staff) {
    await p.refuse("Só quem abriu o ticket ou a equipe pode fechá-lo.");
    return;
  }
  if (closing.has(threadId)) {
    await p.refuse("Este ticket já está sendo fechado.");
    return;
  }
  closing.add(threadId);
  try {
    await p.announce(`Ticket fechado por <@${actor.id}>.`);
    try {
      await p.archive(threadId);
    } catch (e) {
      // Continua aberto no registro: dá para tentar fechar de novo.
      console.error(`arquivar ticket falhou: ${(e as Error).name}`);
      await p.failed("Não consegui arquivar a thread; o ticket continua aberto. Tente de novo.");
      return;
    }
    state.closeTicket(threadId, actor.id, now);
    await p.log(
      logEntry("Ticket fechado", [
        `<#${threadId}>`,
        `Aberto por <@${t.ownerId}>`,
        `Fechado por <@${actor.id}>`,
      ]),
    );
  } finally {
    closing.delete(threadId);
  }
}
