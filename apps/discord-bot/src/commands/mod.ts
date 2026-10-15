/**
 * `/mod`: moderação pela equipe. Duas travas: a permissão do Discord (o comando só aparece para
 * quem pode moderar; `banir` confere também a de banir) e, se configurados, os cargos de
 * `moderation.modRoleIds`. Toda ação vai para o `bot.db` e para o canal de logs. As respostas
 * só quem usou o comando vê.
 */
import { InteractionContextType, PermissionFlagsBits, SlashCommandBuilder } from "discord.js";

import type { BotMessage } from "../format.ts";
import { formatHistory, formatModLog } from "../moderation/actions.ts";
import type { ModerationConfig } from "../moderation/config.ts";
import type { BotState, ModAction } from "../state.ts";
import type { ReplyPayload } from "./nexus.ts";

const MAX_REASON = 300;

export const modCommand = new SlashCommandBuilder()
  .setName("mod")
  .setDescription("Moderação (equipe)")
  .setDefaultMemberPermissions(PermissionFlagsBits.ModerateMembers)
  .setContexts(InteractionContextType.Guild)
  .addSubcommand((s) =>
    s
      .setName("advertir")
      .setDescription("Registra uma advertência")
      .addUserOption((o) => o.setName("usuario").setDescription("Quem").setRequired(true))
      .addStringOption((o) =>
        o.setName("motivo").setDescription("Motivo").setRequired(true).setMaxLength(MAX_REASON),
      ),
  )
  .addSubcommand((s) =>
    s
      .setName("silenciar")
      .setDescription("Silencia (timeout) por alguns minutos")
      .addUserOption((o) => o.setName("usuario").setDescription("Quem").setRequired(true))
      .addIntegerOption((o) =>
        o
          .setName("minutos")
          .setDescription("De 1 minuto a 28 dias")
          .setRequired(true)
          .setMinValue(1)
          .setMaxValue(40_320),
      )
      .addStringOption((o) =>
        o.setName("motivo").setDescription("Motivo").setRequired(true).setMaxLength(MAX_REASON),
      ),
  )
  .addSubcommand((s) =>
    s
      .setName("banir")
      .setDescription("Bane do servidor")
      .addUserOption((o) => o.setName("usuario").setDescription("Quem").setRequired(true))
      .addStringOption((o) =>
        o.setName("motivo").setDescription("Motivo").setRequired(true).setMaxLength(MAX_REASON),
      )
      .addIntegerOption((o) =>
        o
          .setName("apagar_dias")
          .setDescription("Apagar as mensagens dos últimos dias (0 a 7)")
          .setMinValue(0)
          .setMaxValue(7),
      ),
  )
  .addSubcommand((s) =>
    s
      .setName("historico")
      .setDescription("Ações de moderação registradas para alguém")
      .addUserOption((o) => o.setName("usuario").setDescription("Quem").setRequired(true)),
  );

/** A pessoa alvo, como o Discord a vê agora. */
export interface ModTarget {
  id: string;
  isBot: boolean;
  /** Está no servidor (silenciar exige; banir não). */
  inGuild: boolean;
  /** O bot consegue silenciar/banir (cargo abaixo do dele, não é o dono). */
  moderatable: boolean;
  bannable: boolean;
  /**
   * Cargo mais alto da pessoa abaixo do de quem usou o comando (ou quem usou é o dono, ou a
   * pessoa está fora do servidor). Sem isso, um moderador usaria o bot contra quem está acima dele.
   */
  belowActor: boolean;
}

/** O que o comando usa de uma interação (adaptado em `discord.ts`). */
export interface ModInteraction {
  actorId: string;
  actorRoleIds: string[];
  canModerate: boolean;
  canBan: boolean;
  subcommand: string;
  target: ModTarget | null;
  string(name: string): string | null;
  integer(name: string): number | null;
  /** Avisa o Discord que a resposta vem depois (o prazo de 3 s não cobre as chamadas à API). */
  defer(): Promise<unknown>;
  /** Responde, ou edita a resposta adiada. */
  reply(payload: ReplyPayload): Promise<unknown>;
}

export interface ModDeps {
  cfg: ModerationConfig;
  state: BotState;
  timeout(userId: string, ms: number, reason: string): Promise<void>;
  ban(userId: string, reason: string, deleteSeconds: number): Promise<void>;
  log(m: BotMessage): Promise<void>;
  now(): number;
}

const NO_MENTIONS = { parse: [] } as { parse: [] };
const say = (content: string): ReplyPayload => ({
  content,
  allowedMentions: NO_MENTIONS,
  ephemeral: true,
});

export async function handleMod(i: ModInteraction, d: ModDeps): Promise<void> {
  const { cfg, state } = d;
  if (cfg.modRoleIds.length > 0 && !i.actorRoleIds.some((r) => cfg.modRoleIds.includes(r))) {
    await i.reply(say("Só a equipe de moderação pode usar este comando."));
    return;
  }
  if (!i.canModerate) {
    await i.reply(say("Você não tem a permissão de moderar membros."));
    return;
  }
  const t = i.target;
  if (!t) {
    await i.reply(say("Não encontrei essa pessoa."));
    return;
  }
  if (i.subcommand === "historico") {
    const history = formatHistory(t.id, state.history(t.id));
    await i.reply({ embeds: history.embeds, allowedMentions: NO_MENTIONS, ephemeral: true });
    return;
  }
  if (t.id === i.actorId) {
    await i.reply(say("Você não pode aplicar isso a si mesmo."));
    return;
  }
  if (t.isBot) {
    await i.reply(say("Bots não são moderados por este comando."));
    return;
  }
  if (!t.belowActor) {
    await i.reply(say("Você não pode moderar alguém com cargo igual ou acima do seu."));
    return;
  }
  const reason =
    (i.string("motivo") ?? "").replace(/\s+/gu, " ").trim().slice(0, MAX_REASON) ||
    "sem motivo informado";
  await i.defer();
  const at = d.now();
  const record = async (a: ModAction) => {
    state.recordAction(a);
    await d.log(formatModLog(a));
  };

  try {
    switch (i.subcommand) {
      case "advertir": {
        await record({
          userId: t.id,
          by: i.actorId,
          action: "warn",
          reason,
          rule: null,
          minutes: null,
          at,
        });
        const since = at - cfg.warnWindowHours * 3_600_000;
        const n = state.warningsSince(t.id, since);
        await i.reply(say(`Advertência registrada (${n} nas últimas ${cfg.warnWindowHours} h).`));
        return;
      }
      case "silenciar": {
        const minutes = i.integer("minutos") ?? 0;
        if (minutes < 1 || minutes > 40_320) {
          await i.reply(say("A duração vai de 1 minuto a 28 dias."));
          return;
        }
        if (!t.inGuild || !t.moderatable) {
          await i.reply(
            say("Não consigo silenciar essa pessoa (cargo acima do meu ou fora do servidor)."),
          );
          return;
        }
        await d.timeout(t.id, minutes * 60_000, reason);
        await record({
          userId: t.id,
          by: i.actorId,
          action: "timeout",
          reason,
          rule: null,
          minutes,
          at,
        });
        await i.reply(say(`Silenciado por ${minutes} min.`));
        return;
      }
      case "banir": {
        if (!i.canBan) {
          await i.reply(say("Você não tem a permissão de banir membros."));
          return;
        }
        if (t.inGuild && !t.bannable) {
          await i.reply(
            say("Não consigo banir essa pessoa (cargo acima do meu ou dono do servidor)."),
          );
          return;
        }
        const days = i.integer("apagar_dias") ?? 0;
        await d.ban(t.id, reason, days * 86_400);
        await record({
          userId: t.id,
          by: i.actorId,
          action: "ban",
          reason,
          rule: null,
          minutes: null,
          at,
        });
        await i.reply(say("Banido."));
        return;
      }
      default:
        await i.reply(say("Comando desconhecido."));
    }
  } catch (e) {
    // A resposta foi adiada: sem isto, o moderador ficaria vendo "pensando…" até expirar.
    console.error(`/mod falhou: ${(e as Error).name}`);
    await i.reply(
      say("Não consegui concluir a ação. Confira as permissões e a posição do cargo do bot."),
    );
  }
}
