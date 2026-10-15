/**
 * Configuração da moderação (`moderation` em `bot.config.json`). Ausente = moderação desligada,
 * e o bot nem pede as intents privilegiadas. Os padrões são conservadores: só casos claros.
 */

export interface ModerationConfig {
  /** Canal onde cada ação é registrada (opcional). */
  logChannelId: string | null;
  /** Cargos que podem usar `/mod` (além da permissão do Discord). Vazio = só a permissão. */
  modRoleIds: string[];
  /** Automod ligado (precisa da intent Message Content no Developer Portal). */
  automod: boolean;
  /** Canais e cargos que o automod ignora. */
  exemptChannelIds: string[];
  exemptRoleIds: string[];
  /** Flood: mais de `messages` mensagens em `seconds` segundos. */
  flood: { messages: number; seconds: number };
  /** Spam de menções: `max` ou mais menções (pessoas e cargos) numa mensagem. */
  maxMentions: number;
  /** Convites de outros servidores; `allowedInviteCodes` libera os do próprio servidor. */
  blockInvites: boolean;
  allowedInviteCodes: string[];
  /** Advertências do automod em `warnWindowHours` que levam a silenciar por `timeoutMinutes`. */
  warnThreshold: number;
  warnWindowHours: number;
  timeoutMinutes: number;
}

export const MODERATION_DEFAULTS: Omit<
  ModerationConfig,
  "logChannelId" | "modRoleIds" | "exemptChannelIds" | "exemptRoleIds" | "allowedInviteCodes"
> = {
  automod: true,
  flood: { messages: 6, seconds: 8 },
  maxMentions: 6,
  blockInvites: true,
  warnThreshold: 3,
  warnWindowHours: 24,
  timeoutMinutes: 10,
};

const isIdList = (v: unknown): v is string[] =>
  Array.isArray(v) && v.every((x) => typeof x === "string" && /^\d{1,25}$/.test(x));

function intIn(path: string, field: string, v: unknown, min: number, max: number, def: number) {
  if (v === undefined) return def;
  if (typeof v !== "number" || !Number.isInteger(v) || v < min || v > max) {
    throw new Error(`${path}: moderation.${field} deve ser um inteiro de ${min} a ${max}`);
  }
  return v;
}

/** Lê o bloco `moderation`; o erro nomeia o campo inválido. */
export function parseModeration(path: string, raw: unknown): ModerationConfig | null {
  if (raw === undefined || raw === null) return null;
  if (typeof raw !== "object" || Array.isArray(raw)) {
    throw new Error(`${path}: moderation deve ser um objeto`);
  }
  const m = raw as Record<string, unknown>;
  const ids = (field: string): string[] => {
    const v = m[field];
    if (v === undefined) return [];
    if (!isIdList(v)) throw new Error(`${path}: moderation.${field} deve ser uma lista de IDs`);
    return v;
  };
  const log = m.logChannelId;
  if (log !== undefined && log !== null && log !== "" && !isIdList([log])) {
    throw new Error(`${path}: moderation.logChannelId deve ser o ID de um canal (só números)`);
  }
  const flood = (m.flood ?? {}) as Record<string, unknown>;
  if (typeof flood !== "object" || Array.isArray(flood)) {
    throw new Error(`${path}: moderation.flood deve ser um objeto`);
  }
  const codes = m.allowedInviteCodes ?? [];
  if (
    !Array.isArray(codes) ||
    !codes.every((c) => typeof c === "string" && /^[\w-]{2,32}$/.test(c))
  ) {
    throw new Error(`${path}: moderation.allowedInviteCodes deve ser uma lista de códigos`);
  }
  const bool = (field: string, def: boolean) => {
    const v = m[field];
    if (v === undefined) return def;
    if (typeof v !== "boolean")
      throw new Error(`${path}: moderation.${field} deve ser true ou false`);
    return v;
  };
  const d = MODERATION_DEFAULTS;
  return {
    logChannelId: typeof log === "string" && log !== "" ? log : null,
    modRoleIds: ids("modRoleIds"),
    automod: bool("automod", d.automod),
    exemptChannelIds: ids("exemptChannelIds"),
    exemptRoleIds: ids("exemptRoleIds"),
    flood: {
      messages: intIn(path, "flood.messages", flood.messages, 3, 50, d.flood.messages),
      seconds: intIn(path, "flood.seconds", flood.seconds, 2, 120, d.flood.seconds),
    },
    maxMentions: intIn(path, "maxMentions", m.maxMentions, 3, 50, d.maxMentions),
    blockInvites: bool("blockInvites", d.blockInvites),
    allowedInviteCodes: codes as string[],
    warnThreshold: intIn(path, "warnThreshold", m.warnThreshold, 1, 20, d.warnThreshold),
    warnWindowHours: intIn(path, "warnWindowHours", m.warnWindowHours, 1, 720, d.warnWindowHours),
    timeoutMinutes: intIn(path, "timeoutMinutes", m.timeoutMinutes, 1, 40_320, d.timeoutMinutes),
  };
}
