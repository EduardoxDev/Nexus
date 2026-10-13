// Descrição da release a partir do CHANGELOG.md: a seção da versão da tag. Uma versão de teste
// `X.Y.Z-rc.N` usa a seção da versão final `X.Y.Z`, com um aviso; qualquer outra (inclusive
// `-beta`, `-nightly`) precisa da própria seção. No fim, o lembrete do SHA-256 e, se a seção
// ainda não disser, o de pacotes não assinados.
// Uso: node scripts/release-notes.mjs <tag> [CHANGELOG.md]   → imprime em stdout
import { readFileSync } from "node:fs";

const [tag, file = "CHANGELOG.md"] = process.argv.slice(2);
if (!tag) {
  console.error("uso: node scripts/release-notes.mjs <tag> [CHANGELOG.md]");
  process.exit(2);
}

const version = tag.replace(/^v/, "");
const rc = /^(\d+\.\d+\.\d+)-rc\.\d+$/.exec(version);
const heading = rc ? rc[1] : version;
const lines = readFileSync(file, "utf8").split(/\r?\n/);
const start = lines.findIndex((l) => l.trim() === `## ${heading}`);
if (start < 0) {
  console.error(`CHANGELOG sem a seção "## ${heading}"`);
  process.exit(1);
}
const end = lines.findIndex((l, i) => i > start && l.startsWith("## "));
const section = lines
  .slice(start + 1, end < 0 ? undefined : end)
  .join("\n")
  .trim();

const parts = [];
if (rc) {
  parts.push(
    `> **Versão de teste (${version})** da ${heading}. Use para testar antes da versão final.`,
  );
}
parts.push(section);
const footer = ["Confira o SHA-256 dos arquivos em `SHA256SUMS.txt`."];
if (!/não são assinados/i.test(section)) {
  footer.push(
    "Os pacotes não são assinados; veja [como abrir](https://github.com/EduardoxDev/Nexus/blob/main/docs/release.md#abrir-sem-assinatura).",
  );
}
parts.push(`---\n\n${footer.join(" ")}`);
process.stdout.write(`${parts.join("\n\n")}\n`);
