/**
 * Regras do automod, sem Discord: recebem um resumo da mensagem e dizem se ela fere uma regra.
 * Conservadoras de propósito (só casos claros); a decisão de apagar/advertir/silenciar fica em
 * `moderation/actions.ts`.
 */
import type { ModerationConfig } from "./config.ts";

export interface MessageInfo {
  authorId: string;
  /** Mensagens de bots e do sistema não passam pelo automod. */
  authorIsBot: boolean;
  /** Cargos do autor (para isenções). */
  roleIds: string[];
  channelId: string;
  /** Canal pai, quando é uma thread (isenção de canal vale para as threads dele). */
  parentId: string | null;
  content: string;
  /** Pessoas e cargos mencionados (sem repetir), mais `@everyone`/`@here` se houver. */
  mentionCount: number;
  /** Instante da mensagem (ms). */
  at: number;
}

export type Rule = "flood" | "invite" | "mentions";

export interface Verdict {
  rule: Rule;
  /** Motivo mostrado ao autor e no registro (sem copiar o conteúdo da mensagem). */
  reason: string;
}

/** Convites do Discord: `discord.gg/x`, `discord.com/invite/x`, `discordapp.com/invite/x`. */
const INVITE =
  /(?<![\w./@-])(?:https?:\/\/)?(?:www\.)?(?:discord\.gg|discord(?:app)?\.com\/invite)\/([\w-]{2,32})(?![\w-])/giu;

export function inviteCodes(content: string): string[] {
  return [...content.matchAll(INVITE)].map((m) => m[1] ?? "").filter(Boolean);
}

/** Janela deslizante de mensagens por autor, em memória (o flood é de segundos). */
export class FloodTracker {
  private readonly times = new Map<string, number[]>();
  /** Até quando a rajada que já gerou advertência fica sem gerar outra. */
  private readonly quietUntil = new Map<string, number>();

  /** Registra a mensagem e diz quantas o autor mandou dentro da janela (inclusive esta). */
  hit(authorId: string, at: number, windowMs: number): number {
    // Mensagens fora de ordem: as de instante futuro ficam guardadas para as próximas, mas não
    // contam para esta.
    const kept = (this.times.get(authorId) ?? []).filter((t) => t > at || at - t < windowMs);
    kept.push(at);
    this.times.set(authorId, kept);
    const recent = kept.filter((t) => t <= at);
    // Limpa autores parados para a memória não crescer sem limite.
    if (this.times.size > 5_000) {
      for (const [id, list] of this.times) {
        if (list.every((t) => at - t >= windowMs)) this.times.delete(id);
      }
      for (const [id, until] of this.quietUntil) if (until <= at) this.quietUntil.delete(id);
    }
    return recent.length;
  }

  /** Depois de agir, a mesma rajada não gera outra advertência até `until`. */
  quiet(authorId: string, until: number): void {
    this.quietUntil.set(authorId, until);
  }

  isQuiet(authorId: string, at: number): boolean {
    return (this.quietUntil.get(authorId) ?? 0) > at;
  }
}

export function isExempt(cfg: ModerationConfig, m: MessageInfo): boolean {
  return (
    m.authorIsBot ||
    cfg.exemptChannelIds.includes(m.channelId) ||
    (m.parentId !== null && cfg.exemptChannelIds.includes(m.parentId)) ||
    m.roleIds.some((r) => cfg.exemptRoleIds.includes(r))
  );
}

/** A primeira regra ferida, na ordem: convite, menções, flood. */
export function evaluate(
  cfg: ModerationConfig,
  m: MessageInfo,
  flood: FloodTracker,
): Verdict | null {
  if (!cfg.automod || isExempt(cfg, m)) return null;
  if (cfg.blockInvites) {
    const allowed = new Set(cfg.allowedInviteCodes.map((c) => c.toLowerCase()));
    if (inviteCodes(m.content).some((c) => !allowed.has(c.toLowerCase()))) {
      return { rule: "invite", reason: "convite para outro servidor" };
    }
  }
  if (m.mentionCount >= cfg.maxMentions) {
    return { rule: "mentions", reason: `muitas menções numa mensagem (${m.mentionCount})` };
  }
  const windowMs = cfg.flood.seconds * 1000;
  const count = flood.hit(m.authorId, m.at, windowMs);
  if (count > cfg.flood.messages && !flood.isQuiet(m.authorId, m.at)) {
    flood.quiet(m.authorId, m.at + windowMs);
    return {
      rule: "flood",
      reason: `mais de ${cfg.flood.messages} mensagens em ${cfg.flood.seconds} s`,
    };
  }
  return null;
}
