/**
 * Páginas do site. Cada uma é pré-renderizada no build (`scripts/prerender.mjs`) em
 * `dist/<caminho>/index.html`; o conteúdo das páginas de documentação vem de Markdown — do próprio
 * site (`src/content/`) ou do repositório (privacidade e novidades), para não haver duas versões.
 */
import changelog from "../../../CHANGELOG.md?raw";
import privacy from "../../../docs/privacy.md?raw";

import agentes from "./content/agentes.md?raw";
import instalar from "./content/instalar.md?raw";

export interface DocRoute {
  path: string;
  /** Rótulo no menu da documentação. */
  label: string;
  title: string;
  description: string;
  markdown: string;
  /** Arquivo de origem no repositório, para resolver links relativos. */
  source: string;
}

export const HOME_TITLE = "NEXUS — O contexto do seu código, na mão dos seus agentes";

export const DOCS: DocRoute[] = [
  {
    path: "/docs/instalar/",
    label: "Instalar",
    title: "Instalar a NEXUS no Windows, Linux e macOS",
    description:
      "Como baixar, instalar e abrir a NEXUS em cada sistema, inclusive sem assinatura de código, e como funcionam as atualizações.",
    markdown: instalar,
    source: "apps/web/src/content/instalar.md",
  },
  {
    path: "/docs/agentes/",
    label: "Conectar seu agente",
    title: "Conectar a NEXUS ao seu agente de IA",
    description:
      "Configuração do servidor MCP da NEXUS para Claude Code, Codex, Cursor, VS Code e Claude Desktop.",
    markdown: agentes,
    source: "apps/web/src/content/agentes.md",
  },
  {
    path: "/docs/privacidade/",
    label: "Privacidade",
    title: "Privacidade da NEXUS",
    description:
      "O que a NEXUS guarda no seu computador, o que nunca sai dele e o que um agente conectado recebe.",
    markdown: privacy,
    source: "docs/privacy.md",
  },
  {
    path: "/novidades/",
    label: "Novidades",
    title: "Novidades da NEXUS",
    description: "O que mudou em cada versão da NEXUS.",
    markdown: changelog,
    source: "CHANGELOG.md",
  },
];

export const PATHS = ["/", ...DOCS.map((d) => d.path)];

/** Caminho do site sem a base, normalizado (`/docs/instalar` → `/docs/instalar/`). */
export function routeOf(pathname: string, base: string): string {
  let p = pathname.startsWith(base) ? `/${pathname.slice(base.length)}` : pathname;
  p = p.replace(/index\.html$/u, "");
  if (!p.endsWith("/")) p += "/";
  return PATHS.includes(p) ? p : "/";
}

/** Link para uma página do site, com a base (`/` local, `/Nexus/` no GitHub Pages). */
export function pageHref(path: string): string {
  return import.meta.env.BASE_URL + path.replace(/^\//u, "");
}
