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

export function canQuery(cfg: BotConfig, ch: ChannelRef, project: string): boolean {
  const wanted = project.trim().toLowerCase();
  return visibleProjects(cfg, ch).some((p) => p.toLowerCase() === wanted);
}
