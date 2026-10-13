// Descrição da release a partir do CHANGELOG.md: a seção da versão da tag (uma `-rc.N` usa a
// seção da versão final, com um aviso de versão de teste) e o lembrete dos pacotes não assinados.
// Uso: node scripts/release-notes.mjs <tag> [CHANGELOG.md]   → imprime em stdout
import { readFileSync } from "node:fs";

const [tag, file = "CHANGELOG.md"] = process.argv.slice(2);
if (!tag) {
  console.error("uso: node scripts/release-notes.mjs <tag> [CHANGELOG.md]");
  process.exit(2);
}

const version = tag.replace(/^v/, "");
const base = version.replace(/-.*$/, "");
const lines = readFileSync(file, "utf8").split(/\r?\n/);
const start = lines.findIndex((l) => l.trim() === `## ${base}`);
if (start < 0) {
  console.error(`CHANGELOG sem a seção "## ${base}"`);
  process.exit(1);
}
const end = lines.findIndex((l, i) => i > start && l.startsWith("## "));
const section = lines.slice(start + 1, end < 0 ? undefined : end).join("\n").trim();

const parts = [];
if (version !== base) {
  parts.push(
    `> **Versão de teste (${version})** da ${base}. Use para testar antes da versão final.`,
  );
}
parts.push(section);
parts.push(
  "---\n\nConfira o SHA-256 dos arquivos em `SHA256SUMS.txt`. Os pacotes não são assinados; veja [como abrir](https://github.com/EduardoxDev/Nexus/blob/main/docs/release.md#abrir-sem-assinatura).",
);
process.stdout.write(`${parts.join("\n\n")}\n`);
