/** Configuração do bot: segredos e caminhos no ambiente (.env), regras de acesso em JSON. */
import { readFileSync } from "node:fs";

export type Visibility = "public" | "team";

export interface BotConfig {
  /** Projeto aberto (o repositório da NEXUS): consultável em qualquer canal. */
  publicProject: string;
  /** Projetos da equipe: só nos canais de `teamChannelIds` (e threads deles). */
  teamProjects: string[];
  teamChannelIds: string[];
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
  const pub = raw.publicProject.toLowerCase();
  if (raw.teamProjects.some((p) => p.toLowerCase() === pub)) {
    throw new Error(
      `${path}: "${raw.publicProject}" não pode ser público e da equipe ao mesmo tempo`,
    );
  }
  return {
    publicProject: raw.publicProject,
    teamProjects: raw.teamProjects,
    teamChannelIds: raw.teamChannelIds,
  };
}
