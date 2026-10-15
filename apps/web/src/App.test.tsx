import { existsSync } from "node:fs";
import { resolve } from "node:path";

import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { App } from "./App";
import { render as renderStatic } from "./entry-server";

describe("site da NEXUS", () => {
  it("has a single h1 with the promise", () => {
    render(<App />);
    const h1 = screen.getAllByRole("heading", { level: 1 });
    expect(h1).toHaveLength(1);
    expect(h1[0]).toHaveAccessibleName("O contexto do seu código, na mão do seu agente de IA.");
  });

  it("every in-page link points to an existing section", () => {
    const { container } = render(<App />);
    const anchors = [...container.querySelectorAll<HTMLAnchorElement>('a[href^="#"]')];
    expect(anchors.length).toBeGreaterThan(5);
    for (const a of anchors) {
      const id = a.getAttribute("href")!.slice(1);
      if (id === "") continue; // "#" = topo
      expect(container.querySelector(`#${id}`), `âncora #${id}`).not.toBeNull();
    }
  });

  it("never renders empty or placeholder links", () => {
    const { container } = render(<App />);
    for (const a of container.querySelectorAll("a")) {
      const href = a.getAttribute("href");
      expect(href, a.textContent ?? "").toBeTruthy();
      expect(href).not.toMatch(/undefined|null|TODO/);
    }
  });

  it("every image has a description or is marked decorative", () => {
    const { container } = render(<App />);
    for (const img of container.querySelectorAll("img")) {
      expect(img.hasAttribute("alt"), img.getAttribute("src") ?? "").toBe(true);
    }
    expect(screen.getAllByRole("img", { name: /Busca com motivos/ }).length).toBeGreaterThan(0);
  });

  it("no button or link is hidden from screen readers", () => {
    const { container } = render(<App />);
    for (const el of container.querySelectorAll("a, button")) {
      expect(el.closest('[aria-hidden="true"]'), el.textContent ?? "").toBeNull();
    }
  });

  it("every image file the page references exists in public/", () => {
    const { container } = render(<App />);
    const srcs = [...container.querySelectorAll("img")].map((i) => i.getAttribute("src")!);
    expect(srcs).toEqual(expect.arrayContaining(["/logo.png", "/screens/busca-com-previa.webp"]));
    for (const src of new Set(srcs)) {
      expect(existsSync(resolve(process.cwd(), "public", src.slice(1))), src).toBe(true);
    }
  });

  it("main navigation reaches the key sections", () => {
    render(<App />);
    const nav = screen.getByRole("navigation", { name: "Principal" });
    for (const name of ["O que faz", "Como começar", "Privacidade", "Perguntas"]) {
      expect(within(nav).getByRole("link", { name })).toBeInTheDocument();
    }
  });

  it("prerendered HTML carries the content (search engines and no-JS visitors)", () => {
    const { html } = renderStatic();
    expect(html).toContain("Seu código fica no seu computador");
    expect(html).toContain("Perguntas que sempre aparecem");
    expect(html).toContain("O que ela faz por você");
    expect(html).toContain("claude mcp add nexus -- nexus mcp serve");
  });
});
