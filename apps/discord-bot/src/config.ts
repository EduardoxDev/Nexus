/** Configuração do bot: segredos e caminhos no ambiente (.env), regras de acesso em JSON. */
import { readFileSync } from "node:fs";

import { sameProject } from "./access.ts";
import { parseManagement, type ManagementConfig } from "./management/config.ts";
import { parseModeration, type ModerationConfig } from "./moderation/config.ts";

export type Visibility = "public" | "team";

export interface BotConfig {
  /** Projeto aberto (o repositório da NEXUS): consultável em qualquer canal. */
  publicProject: string;
  /** Projetos da equipe: só nos canais de `teamChannelIds` (e threads deles). */
  teamProjects: string[];
  teamChannelIds: string[];
  /** Canal onde versões novas são anunciadas (opcional: sem ele, não anuncia). */
  announceChannelId: string | null;
  /** Canal da equipe para avisos internos da NEXUS (opcional). Precisa ser canal da equipe. */
  teamAlertsChannelId: string | null;
  /** Repositório das releases, `dono/nome`. */
  githubRepo: string;
  /** Moderação (automod e `/mod`). `null` = desligada. */
  moderation: ModerationConfig | null;
  /** Boas-vindas, cargos por botão e tickets. `null` = desligados. */
  management: ManagementConfig | null;
}

const DEFAULT_REPO = "EduardoxDev/Nexus";

/** ID de canal opcional: ausente/vazio = desligado; senão só dígitos. */
function channelId(path: string, field: string, v: unknown): string | null {
  if (v === undefined || v === null || v === "") return null;
  if (typeof v !== "string" || !/^\d{1,25}$/.test(v)) {
    throw new Error(`${path}: ${field} deve ser o ID de um canal (só números)`);
  }
  return v;
}

export interface Env {
  token: string;
  appId: string;
  guildId: string;
  nexusCli: string;
  nexusDataDir: string;
}

const ENV_KEYS = {
  token: "DISCORD_TOKEN",
  appId: "DISCORD_APP_ID",
  guildId: "DISCORD_GUILD_ID",
  nexusCli: "NEXUS_CLI",
  nexusDataDir: "NEXUS_DATA_DIR",
} as const satisfies Record<keyof Env, string>;

/** Lê as variáveis obrigatórias. O erro lista só os nomes faltantes, nunca valores. */
export function loadEnv(env: NodeJS.ProcessEnv): Env {
  const missing = Object.values(ENV_KEYS).filter((k) => !env[k]?.trim());
  if (missing.length > 0) {
    throw new Error(`variáveis de ambiente faltando: ${missing.join(", ")} (veja .env.example)`);
  }
  const out = {} as Env;
  for (const [field, key] of Object.entries(ENV_KEYS)) {
    out[field as keyof Env] = (env[key] ?? "").trim();
  }
  return out;
}

const isStringArray = (v: unknown): v is string[] =>
  Array.isArray(v) && v.every((x) => typeof x === "string");

/** Lê e valida `bot.config.json`; o erro nomeia o campo inválido. */
export function loadConfig(path: string): BotConfig {
  const raw = JSON.parse(readFileSync(path, "utf8")) as Record<string, unknown>;
  if (typeof raw.publicProject !== "string" || !raw.publicProject.trim()) {
    throw new Error(`${path}: publicProject deve ser o nome do projeto público`);
  }
  if (!isStringArray(raw.teamProjects)) {
    throw new Error(`${path}: teamProjects deve ser uma lista de nomes de projetos`);
  }
  if (!isStringArray(raw.teamChannelIds)) {
    throw new Error(`${path}: teamChannelIds deve ser uma lista de IDs de canais`);
  }
  const pub = raw.publicProject;
  if (raw.teamProjects.some((p) => sameProject(p, pub))) {
    throw new Error(
      `${path}: "${raw.publicProject}" não pode ser público e da equipe ao mesmo tempo`,
    );
  }
  const teamAlertsChannelId = channelId(path, "teamAlertsChannelId", raw.teamAlertsChannelId);
  // Avisos falam de projetos da equipe: só podem ir para um canal da equipe.
  if (teamAlertsChannelId !== null && !raw.teamChannelIds.includes(teamAlertsChannelId)) {
    throw new Error(`${path}: teamAlertsChannelId precisa estar também em teamChannelIds`);
  }
  const repo = raw.githubRepo ?? DEFAULT_REPO;
  if (typeof repo !== "string" || !/^[\w.-]+\/[\w.-]+$/.test(repo)) {
    throw new Error(`${path}: githubRepo deve ser "dono/nome"`);
  }
  return {
    publicProject: raw.publicProject,
    teamProjects: raw.teamProjects,
    teamChannelIds: raw.teamChannelIds,
    announceChannelId: channelId(path, "announceChannelId", raw.announceChannelId),
    teamAlertsChannelId,
    githubRepo: repo,
    moderation: parseModeration(path, raw.moderation),
    management: parseManagement(path, raw.management),
  };
}
