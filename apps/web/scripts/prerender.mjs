// Pré-renderiza cada página: o HTML final já traz todo o conteúdo (bom para buscadores e para quem
// abre sem JavaScript); no navegador, o React só assume a página (hydrate).
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { pathToFileURL } from "node:url";

// Endereço público do site (para og:url). Padrão: o GitHub Pages; outro destino passa SITE_URL.
const SITE_URL = (process.env.SITE_URL ?? "https://eduardoxdev.github.io/Nexus").replace(
  /\/+$/u,
  "",
);

const root = resolve(import.meta.dirname, "..");
const { render, PATHS } = await import(
  pathToFileURL(resolve(root, "dist-ssr", "entry-server.js")).href
);
const template = readFileSync(resolve(root, "dist", "index.html"), "utf8");
if (!template.includes("<!--app-->"))
  throw new Error("marcador <!--app--> não encontrado em dist/index.html");

/** Atributo HTML: o título e a descrição vêm do código, mas escapamos mesmo assim. */
const attr = (s) =>
  s
    .replaceAll("&", "&amp;")
    .replaceAll('"', "&quot;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");

function swap(html, re, value, what) {
  if (!re.test(html)) throw new Error(`${what} não encontrado em dist/index.html`);
  return html.replace(re, value);
}

for (const path of PATHS) {
  const page = render(path);
  let html = template.replace("<!--app-->", () => page.html);
  html = swap(html, /<title>[^<]*<\/title>/u, `<title>${attr(page.title)}</title>`, "<title>");
  html = swap(
    html,
    /(<meta\s+name="description"\s+content=")[^"]*(")/u,
    `$1${attr(page.description)}$2`,
    "meta description",
  );
  html = swap(
    html,
    /(<meta property="og:url" content=")[^"]*(")/u,
    `$1${SITE_URL}${path}$2`,
    "og:url",
  );
  if (path !== "/") {
    html = swap(
      html,
      /(<meta property="og:title" content=")[^"]*(")/u,
      `$1${attr(page.title)}$2`,
      "og:title",
    );
    html = swap(
      html,
      /(<meta\s+property="og:description"\s+content=")[^"]*(")/u,
      `$1${attr(page.description)}$2`,
      "og:description",
    );
  }
  const file = resolve(root, "dist", `.${path}`, "index.html");
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, html);
  console.log(`pré-renderizado: dist${path}index.html`);
}
rmSync(resolve(root, "dist-ssr"), { recursive: true, force: true });
