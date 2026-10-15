/** Registra os comandos de barra no servidor configurado (aparecem na hora). */
import { existsSync } from "node:fs";
import { resolve } from "node:path";

import { REST, Routes } from "discord.js";

import { modCommand } from "./commands/mod.ts";
import { nexusCommand } from "./commands/nexus.ts";
import { panelCommand, ticketCommand } from "./commands/panel.ts";
import { loadEnv } from "./config.ts";

const envFile = resolve(import.meta.dirname, "..", ".env");
if (existsSync(envFile)) process.loadEnvFile(envFile);
const env = loadEnv(process.env);

const rest = new REST().setToken(env.token);
await rest.put(Routes.applicationGuildCommands(env.appId, env.guildId), {
  body: [nexusCommand.toJSON(), modCommand.toJSON(), panelCommand.toJSON(), ticketCommand.toJSON()],
});
console.log("Comandos registrados: /nexus, /mod, /painel, /ticket");
