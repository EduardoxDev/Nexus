/**
 * Configuração da gestão do servidor (`management` em `bot.config.json`): boas-vindas, cargos
 * por botão e tickets. Cada parte é opcional; o bloco ausente desliga tudo.
 */

export interface WelcomeConfig {
  channelId: string;
  /** Texto do embed; `{usuario}` vira a menção da pessoa e `{servidor}` o nome do servidor. */
  message: string;
}

export interface RoleOption {
  roleId: string;
  /** Texto do botão. */
  label: string;
  emoji: string | null;
}

export interface TicketsConfig {
  /** Canal de texto onde as threads privadas dos tickets são criadas. */
  channelId: string;
  /** Cargos da equipe, chamados para cada ticket novo e que podem fechar qualquer ticket. */
  staffRoleIds: string[];
  /** Registro de abertura/fechamento (opcional). */
  logChannelId: string | null;
}

export interface ManagementConfig {
  welcome: WelcomeConfig | null;
  /** Cargos que o bot pode dar e tirar pelos botões. Só estes, nunca outros. */
  roles: RoleOption[];
  tickets: TicketsConfig | null;
}

export const DEFAULT_WELCOME =
  "Que bom ter você aqui, {usuario}! Dá uma olhada nas regras e fique à vontade para perguntar.";
/** Botões numa mensagem: 5 fileiras de 5. */
export const MAX_ROLE_BUTTONS = 25;

const ID = /^\d{1,25}$/;

/** Emoji de botão como o Discord aceita: um emoji Unicode ou um do servidor (`<:nome:id>`). */
export interface ButtonEmoji {
  name: string;
  id?: string;
  animated?: boolean;
}

const UNICODE_EMOJI =
  /^(?:\p{Regional_Indicator}{2}|[#*0-9]\uFE0F?\u20E3|\p{Extended_Pictographic}(?:\uFE0F|\p{Emoji_Modifier})?(?:\u200D\p{Extended_Pictographic}(?:\uFE0F|\p{Emoji_Modifier})?)*)$/u;
const CUSTOM_EMOJI = /^<(a?):(\w{2,32}):(\d{1,25})>$/u;

/** `null` se não for um emoji que o Discord aceite num botão. */
export function buttonEmoji(s: string): ButtonEmoji | null {
  const custom = CUSTOM_EMOJI.exec(s);
  if (custom) {
    return { name: custom[2] ?? "", id: custom[3] ?? "", ...(custom[1] ? { animated: true } : {}) };
  }
  return UNICODE_EMOJI.test(s) ? { name: s } : null;
}

function obj(path: string, field: string, v: unknown): Record<string, unknown> {
  if (typeof v !== "object" || v === null || Array.isArray(v)) {
    throw new Error(`${path}: ${field} deve ser um objeto`);
  }
  return v as Record<string, unknown>;
}

function id(path: string, field: string, v: unknown): string {
  if (typeof v !== "string" || !ID.test(v)) {
    throw new Error(`${path}: ${field} deve ser um ID do Discord (só números)`);
  }
  return v;
}

function optionalId(path: string, field: string, v: unknown): string | null {
  return v === undefined || v === null || v === "" ? null : id(path, field, v);
}

function ids(path: string, field: string, v: unknown): string[] {
  if (v === undefined) return [];
  if (!Array.isArray(v)) throw new Error(`${path}: ${field} deve ser uma lista de IDs`);
  return v.map((x, n) => id(path, `${field}[${n}]`, x));
}

/** Lê o bloco `management`; o erro nomeia o campo inválido. */
export function parseManagement(path: string, raw: unknown): ManagementConfig | null {
  if (raw === undefined || raw === null) return null;
  const m = obj(path, "management", raw);

  let welcome: WelcomeConfig | null = null;
  if (m.welcome !== undefined && m.welcome !== null) {
    const w = obj(path, "management.welcome", m.welcome);
    const message = w.message ?? DEFAULT_WELCOME;
    if (typeof message !== "string" || !message.trim() || message.length > 2_000) {
      throw new Error(`${path}: management.welcome.message deve ter de 1 a 2000 caracteres`);
    }
    welcome = { channelId: id(path, "management.welcome.channelId", w.channelId), message };
  }

  const rawRoles = m.roles ?? [];
  if (!Array.isArray(rawRoles) || rawRoles.length > MAX_ROLE_BUTTONS) {
    throw new Error(
      `${path}: management.roles deve ser uma lista de até ${MAX_ROLE_BUTTONS} cargos`,
    );
  }
  const roles = rawRoles.map((r, n): RoleOption => {
    const f = `management.roles[${n}]`;
    const o = obj(path, f, r);
    const label = o.label;
    if (typeof label !== "string" || !label.trim() || label.length > 80) {
      throw new Error(`${path}: ${f}.label deve ter de 1 a 80 caracteres`);
    }
    const emoji = o.emoji ?? null;
    if (emoji !== null && (typeof emoji !== "string" || buttonEmoji(emoji) === null)) {
      throw new Error(`${path}: ${f}.emoji deve ser um emoji (Unicode ou <:nome:id>)`);
    }
    return { roleId: id(path, `${f}.roleId`, o.roleId), label: label.trim(), emoji };
  });
  if (new Set(roles.map((r) => r.roleId)).size !== roles.length) {
    throw new Error(`${path}: management.roles tem o mesmo cargo duas vezes`);
  }

  let tickets: TicketsConfig | null = null;
  if (m.tickets !== undefined && m.tickets !== null) {
    const t = obj(path, "management.tickets", m.tickets);
    tickets = {
      channelId: id(path, "management.tickets.channelId", t.channelId),
      staffRoleIds: ids(path, "management.tickets.staffRoleIds", t.staffRoleIds),
      logChannelId: optionalId(path, "management.tickets.logChannelId", t.logChannelId),
    };
  }
  return { welcome, roles, tickets };
}
