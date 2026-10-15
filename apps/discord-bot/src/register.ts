/** Registra os comandos de barra no servidor configurado (aparecem na hora). */
import { existsSync } from "node:fs";
import { resolve } from "node:path";

import { REST, Routes } from "discord.js";

import { modCommand } from "./commands/mod.ts";
import { nexusCommand } from "./commands/nexus.ts";
import { panelCommand, ticketCommand } from "./commands/panel.ts";
import { loadConfig, loadEnv } from "./config.ts";

const root = resolve(import.meta.dirname, "..");
const envFile = resolve(root, ".env");
if (existsSync(envFile)) process.loadEnvFile(envFile);
const env = loadEnv(process.env);
const cfg = loadConfig(resolve(root, "bot.config.json"));

// Só os comandos das partes configuradas; registrar de novo remove os que foram desligados.
const commands = [
  nexusCommand,
  ...(cfg.moderation ? [modCommand] : []),
  ...(cfg.management && (cfg.management.roles.length > 0 || cfg.management.tickets)
    ? [panelCommand]
    : []),
  ...(cfg.management?.tickets ? [ticketCommand] : []),
];

const rest = new REST().setToken(env.token);
await rest.put(Routes.applicationGuildCommands(env.appId, env.guildId), {
  body: commands.map((c) => c.toJSON()),
});
console.log(`Comandos registrados: ${commands.map((c) => `/${c.name}`).join(", ")}`);
