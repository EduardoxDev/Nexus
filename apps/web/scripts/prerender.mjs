// Pré-renderiza a página: o HTML final já traz todo o conteúdo (bom para buscadores e para quem
// abre sem JavaScript); no navegador, o React só assume a página (hydrate).
import { readFileSync, rmSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const root = resolve(import.meta.dirname, "..");
const { render } = await import(pathToFileURL(resolve(root, "dist-ssr", "entry-server.js")).href);
const file = resolve(root, "dist", "index.html");
const html = readFileSync(file, "utf8");
if (!html.includes("<!--app-->"))
  throw new Error("marcador <!--app--> não encontrado em dist/index.html");
writeFileSync(file, html.replace("<!--app-->", render()));
rmSync(resolve(root, "dist-ssr"), { recursive: true, force: true });
console.log("pré-renderizado: dist/index.html");
