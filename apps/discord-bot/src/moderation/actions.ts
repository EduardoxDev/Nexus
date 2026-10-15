/**
 * O que acontece quando o automod pega uma mensagem: apaga, adverte, e — se a pessoa já tem
 * advertências demais na janela — silencia (timeout). Tudo é registrado no `bot.db` e, se houver,
 * no canal de logs. O Discord fica atrás de `AutomodPorts` (os testes usam portas falsas).
 */
import { escapeMarkdown } from "discord.js";

import type { BotMessage } from "../format.ts";
import type { BotState, ModAction } from "../state.ts";
import type { MessageInfo, Verdict } from "./automod.ts";
import type { ModerationConfig } from "./config.ts";

export interface AutomodPorts {
  deleteMessage(): Promise<void>;
  /** Aviso curto no canal, mencionando só a pessoa. */
  notify(userId: string, text: string): Promise<void>;
  timeout(userId: string, ms: number, reason: string): Promise<void>;
  /** Registro no canal de logs (sem canal configurado, não faz nada). */
  log(m: BotMessage): Promise<void>;
}

export type AutomodOutcome = "warned" | "timed_out";

const ACTION_TITLE: Record<ModAction["action"], string> = {
  warn: "Advertência",
  timeout: "Silenciado",
  ban: "Banido",
};
const ACTION_COLOR: Record<ModAction["action"], number> = {
  warn: 0xf5a524,
  timeout: 0xf97316,
  ban: 0xef4444,
};

/** Motivo digitado por moderador: uma linha, sem Markdown (não forja formatação nem registros). */
const plain = (s: string, max: number) =>
  escapeMarkdown(s.replace(/\s+/gu, " ").trim().slice(0, max)).replace(/[<>@]/gu, (c) => `\\${c}`);

/** Registro de uma ação no canal de logs. Menções num embed não notificam ninguém. */
export function formatModLog(a: ModAction): BotMessage {
  const fields = [
    { name: "Pessoa", value: `<@${a.userId}>`, inline: true },
    { name: "Por", value: a.by === "automod" ? "automod" : `<@${a.by}>`, inline: true },
    { name: "Motivo", value: plain(a.reason, 900) || "—" },
  ];
  if (a.rule) fields.push({ name: "Regra", value: a.rule, inline: true });
  if (a.minutes !== null) fields.push({ name: "Duração", value: `${a.minutes} min`, inline: true });
  return {
    embeds: [
      {
        title: ACTION_TITLE[a.action],
        color: ACTION_COLOR[a.action],
        fields,
        timestamp: new Date(a.at).toISOString(),
      },
    ],
    files: [],
  };
}

/** Histórico de uma pessoa (resposta de `/mod historico`). */
export function formatHistory(userId: string, actions: ModAction[]): BotMessage {
  if (actions.length === 0) {
    return {
      embeds: [{ description: `Nenhuma ação registrada para <@${userId}>.` }],
      files: [],
    };
  }
  const lines = actions.map((a) => {
    const when = `<t:${Math.floor(a.at / 1000)}:R>`;
    const by = a.by === "automod" ? "automod" : `<@${a.by}>`;
    const extra = a.minutes !== null ? ` (${a.minutes} min)` : "";
    return `• **${ACTION_TITLE[a.action]}**${extra} ${when} por ${by}: ${plain(a.reason, 200)}`;
  });
  return {
    embeds: [
      { title: "Histórico de moderação", description: `<@${userId}>\n\n${lines.join("\n")}` },
    ],
    files: [],
  };
}

export async function applyAutomod(
  cfg: ModerationConfig,
  state: BotState,
  m: MessageInfo,
  v: Verdict,
  ports: AutomodPorts,
): Promise<AutomodOutcome> {
  try {
    await ports.deleteMessage();
  } catch (e) {
    // Sem permissão para apagar (ou já apagada): a advertência continua valendo.
    console.error(`automod: não foi possível apagar a mensagem: ${(e as Error).name}`);
  }
  const warn: ModAction = {
    userId: m.authorId,
    by: "automod",
    action: "warn",
    reason: v.reason,
    rule: v.rule,
    minutes: null,
    at: m.at,
  };
  state.recordAction(warn);
  await ports.log(formatModLog(warn));
  const since = m.at - cfg.warnWindowHours * 3_600_000;
  const warnings = state.warningsSince(m.authorId, since);
  if (warnings >= cfg.warnThreshold) {
    const reason = `${warnings} advertências em ${cfg.warnWindowHours} h (última: ${v.reason})`;
    let timedOut = false;
    try {
      await ports.timeout(m.authorId, cfg.timeoutMinutes * 60_000, reason);
      timedOut = true;
    } catch (e) {
      // Cargo acima do bot ou sem permissão: fica a advertência; a equipe vê no registro.
      console.error(`automod: não foi possível silenciar: ${(e as Error).name}`);
    }
    if (timedOut) {
      const timeout: ModAction = {
        userId: m.authorId,
        by: "automod",
        action: "timeout",
        reason,
        rule: v.rule,
        minutes: cfg.timeoutMinutes,
        at: m.at,
      };
      state.recordAction(timeout);
      await ports.log(formatModLog(timeout));
      await notify(
        ports,
        m.authorId,
        `<@${m.authorId}>, você foi silenciado por ${cfg.timeoutMinutes} min (${v.reason}).`,
      );
      return "timed_out";
    }
  }
  await notify(
    ports,
    m.authorId,
    `<@${m.authorId}>, sua mensagem foi removida: ${v.reason}. (${warnings}/${cfg.warnThreshold})`,
  );
  return "warned";
}

/** O aviso no canal é cortesia: se falhar, a ação já valeu e está registrada. */
async function notify(ports: AutomodPorts, userId: string, text: string): Promise<void> {
  try {
    await ports.notify(userId, text);
  } catch (e) {
    console.error(`automod: não foi possível avisar no canal: ${(e as Error).name}`);
  }
}
