import { existsSync } from "node:fs";
import { resolve } from "node:path";

import { render, screen, within } from "@testing-library/react";
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { App } from "./App";
import { render as renderStatic } from "./entry-server";
import { inline, Markdown, parseBlocks, resolveHref, slug } from "./lib/markdown";
import { DOCS, PATHS, routeOf } from "./routes";

const ctx = { source: "docs/privacy.md", base: "/", repo: "https://github.com/dono/repo" };
const html = (md: string) => renderToString(<Markdown blocks={parseBlocks(md)} ctx={ctx} />);

describe("markdown", () => {
  it("renders headings with anchors, paragraphs, lists and sublists", () => {
    const out = html(
      "# Título\n\nUm **forte** e `código`.\n\n- a\n  continua\n- b\n   - sub\n\n1. um\n2. dois",
    );
    expect(out).toContain('<h1 id="titulo"');
    expect(out).toContain("<strong");
    expect(out).toContain("<code");
    expect(out).toContain("a continua");
    expect(out).toMatch(/<ul[^>]*>.*<li[^>]*>sub<\/li>/su);
    expect(out).toContain("<ol");
  });

  it("renders tables and code blocks without interpreting them", () => {
    const out = html('| A | B |\n| - | - |\n| `x \\| y` | **z** |\n\n```json\n{ "a": "<b>" }\n```');
    expect(out).toContain("<table");
    expect(out).toContain("x | y");
    expect(out).toContain("&lt;b&gt;");
    expect(out).not.toContain("<b>");
  });

  it("always moves forward, even on lines no block accepts", () => {
    expect(parseBlocks("| sem tabela\nfim").map((b) => b.t)).toEqual(["p"]);
    expect(parseBlocks("```\nsem fim").map((b) => b.t)).toEqual(["code"]);
  });

  it("never renders raw HTML from the text", () => {
    expect(html("<script>alert(1)</script> <img src=x onerror=y>")).not.toMatch(/<script|<img/u);
  });

  it("links: external kept, site pages get the base, repository files go to GitHub, scripts dropped", () => {
    expect(resolveHref("https://x.dev", ctx)).toBe("https://x.dev");
    expect(resolveHref("#a", ctx)).toBe("#a");
    expect(resolveHref("/docs/agentes/", { ...ctx, base: "/Nexus/" })).toBe("/Nexus/docs/agentes/");
    expect(resolveHref("decisions/ADR-1.md#x", ctx)).toBe(
      "https://github.com/dono/repo/blob/main/docs/decisions/ADR-1.md#x",
    );
    expect(resolveHref("../CHANGELOG.md", ctx)).toBe(
      "https://github.com/dono/repo/blob/main/CHANGELOG.md",
    );
    expect(resolveHref("javascript:alert(1)", ctx)).toBeNull();
    expect(renderToString(<>{inline("[x](javascript:alert(1))", ctx)}</>)).not.toContain("href");
  });

  it("slugs headings like GitHub and keeps them unique", () => {
    expect(slug("Histórico do git")).toBe("historico-do-git");
    const ids = parseBlocks("## A\n\n## A").map((b) => (b.t === "h" ? b.id : ""));
    expect(ids).toEqual(["a", "a-1"]);
  });
});

describe("páginas de documentação", () => {
  it("normalizes paths with or without the base", () => {
    expect(routeOf("/Nexus/docs/instalar", "/Nexus/")).toBe("/docs/instalar/");
    expect(routeOf("/Nexus/novidades/index.html", "/Nexus/")).toBe("/novidades/");
    expect(routeOf("/Nexus/", "/Nexus/")).toBe("/");
    expect(routeOf("/nao-existe/", "/")).toBe("/");
  });

  for (const doc of DOCS) {
    it(`${doc.path}: one h1, the docs menu and the content`, () => {
      render(<App path={doc.path} />);
      expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
      const menu = screen.getByRole("navigation", { name: "Documentação" });
      expect(within(menu).getByRole("link", { name: doc.label })).toHaveAttribute(
        "aria-current",
        "page",
      );
      for (const d of DOCS)
        expect(within(menu).getByRole("link", { name: d.label })).toBeInTheDocument();
    });
  }

  it("every link on every page goes somewhere that exists", () => {
    const home = render(<App path="/" />).container;
    const homeIds = new Set([...home.querySelectorAll("[id]")].map((e) => e.id));
    home.remove();
    for (const path of PATHS) {
      const { container, unmount } = render(<App path={path} />);
      for (const a of container.querySelectorAll("a")) {
        const href = a.getAttribute("href") ?? "";
        const where = `${path} → ${href} (${a.textContent})`;
        expect(href, where).toBeTruthy();
        // Dois testes separados: nenhum esquema perigoso no início, nem "undefined"/"null" em lugar nenhum.
        expect(href, where).not.toMatch(/^\s*(?:javascript|data|vbscript):/iu);
        expect(href, where).not.toMatch(/(?:undefined|null)/u);
        if (/^(https?:|mailto:)/u.test(href)) continue;
        if (href.startsWith("#")) {
          if (href !== "#")
            expect(container.querySelector(`[id="${href.slice(1)}"]`), where).not.toBeNull();
          continue;
        }
        const [page = "", hash] = href.split("#");
        if (/\.[a-z0-9]+$/iu.test(page)) {
          expect(existsSync(resolve(process.cwd(), "public", page.slice(1))), where).toBe(true);
          continue;
        }
        expect(PATHS, where).toContain(page);
        if (hash && page === "/") expect(homeIds.has(hash), where).toBe(true);
      }
      unmount();
    }
  });

  it("prerendered pages carry their own content and title", () => {
    const install = renderStatic("/docs/instalar/");
    expect(install.html).toContain("xattr -dr com.apple.quarantine");
    expect(install.title).toMatch(/Instalar/u);
    expect(renderStatic("/docs/agentes/").html).toContain("codex mcp add nexus");
    expect(renderStatic("/docs/privacidade/").html).toContain("Nada sai da sua máquina");
    expect(renderStatic("/novidades/").html).toContain("0.2.0");
    expect(renderStatic("/").html).toContain("Perguntas que sempre aparecem");
  });
});
