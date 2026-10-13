// Gera o latest.json do atualizador do Tauri a partir dos pacotes assinados de uma release.
// Uso: node scripts/latest-json.mjs <pasta com os pacotes> <tag> <dono/repo>
//
// Plataformas (cada uma precisa do pacote e do .sig ao lado):
//   windows-x86_64  NEXUS_<v>_x64-setup.exe
//   linux-x86_64    NEXUS_<v>_amd64.AppImage   (o .deb não se atualiza sozinho)
//   darwin-aarch64  NEXUS_aarch64.app.tar.gz
//   darwin-x86_64   NEXUS_x64.app.tar.gz
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const [dir, tag, repo] = process.argv.slice(2);
if (!dir || !tag || !repo) {
  console.error("uso: node scripts/latest-json.mjs <pasta> <tag> <dono/repo>");
  process.exit(2);
}

const PLATFORMS = [
  ["windows-x86_64", /_x64-setup\.exe$/],
  ["linux-x86_64", /\.AppImage$/],
  ["darwin-aarch64", /_aarch64\.app\.tar\.gz$/],
  ["darwin-x86_64", /_x64\.app\.tar\.gz$/],
];

const files = readdirSync(dir);
const platforms = {};
for (const [platform, pattern] of PLATFORMS) {
  const file = files.find((f) => pattern.test(f));
  if (!file) {
    console.warn(`aviso: sem pacote para ${platform}`);
    continue;
  }
  const sig = join(dir, `${file}.sig`);
  if (!existsSync(sig)) {
    console.error(`falta a assinatura ${file}.sig (TAURI_SIGNING_PRIVATE_KEY configurada?)`);
    process.exit(1);
  }
  platforms[platform] = {
    signature: readFileSync(sig, "utf8").trim(),
    url: `https://github.com/${repo}/releases/download/${encodeURIComponent(tag)}/${encodeURIComponent(file)}`,
  };
}
if (!platforms["windows-x86_64"]) {
  console.error("o pacote do Windows é obrigatório");
  process.exit(1);
}

const latest = {
  version: tag.replace(/^v/, ""),
  notes: `Notas completas: https://github.com/${repo}/releases/tag/${tag}`,
  pub_date: new Date().toISOString(),
  platforms,
};
writeFileSync(join(dir, "latest.json"), `${JSON.stringify(latest, null, 2)}\n`);
console.log(`latest.json: ${latest.version} para ${Object.keys(platforms).join(", ")}`);
