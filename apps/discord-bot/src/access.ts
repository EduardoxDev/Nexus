/**
 * Regra central do bot: projetos da equipe nunca são consultados, listados ou citados fora dos
 * canais da equipe. Threads herdam o acesso do canal pai.
 */
import type { BotConfig } from "./config.ts";

export interface ChannelRef {
  id: string;
  /** Canal pai, quando é uma thread. */
  parentId: string | null;
}

/**
 * Mesmo nome de projeto, como a NEXUS compara (`eq_ignore_ascii_case`): só A–Z ignoram caixa.
 * O `toLowerCase` do JavaScript dobraria outras letras (ex.: "K" de Kelvin → "k") e aceitaria
 * nomes que a NEXUS não reconhece.
 */
export function sameProject(a: string, b: string): boolean {
  const ascii = (s: string) => s.trim().replace(/[A-Z]/g, (c) => c.toLowerCase());
  return ascii(a) === ascii(b);
}

export function isTeamChannel(cfg: BotConfig, ch: ChannelRef): boolean {
  return (
    cfg.teamChannelIds.includes(ch.id) ||
    (ch.parentId !== null && cfg.teamChannelIds.includes(ch.parentId))
  );
}

/** Projetos consultáveis neste canal, o público primeiro. */
export function visibleProjects(cfg: BotConfig, ch: ChannelRef): string[] {
  return isTeamChannel(cfg, ch) ? [cfg.publicProject, ...cfg.teamProjects] : [cfg.publicProject];
}

/** O projeto com a grafia da configuração, se puder ser consultado neste canal. */
export function findProject(cfg: BotConfig, ch: ChannelRef, asked: string): string | null {
  return visibleProjects(cfg, ch).find((p) => sameProject(p, asked)) ?? null;
}

export function canQuery(cfg: BotConfig, ch: ChannelRef, project: string): boolean {
  return findProject(cfg, ch, project) !== null;
}
