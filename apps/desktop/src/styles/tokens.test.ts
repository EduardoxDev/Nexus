/**
 * Verifica o contraste WCAG dos tokens reais (tokens.css) nos dois temas.
 * Converte OKLCH → sRGB linear (fórmulas de Björn Ottosson) e calcula a luminância relativa.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

// O Vitest roda com cwd = apps/desktop.
const css = readFileSync(resolve(process.cwd(), "src/styles/tokens.css"), "utf8");

type Theme = Record<string, string>;

function parseBlock(selector: RegExp): Theme {
  const match = selector.exec(css);
  if (!match) throw new Error(`bloco não encontrado: ${String(selector)}`);
  const vars: Theme = {};
  for (const m of match[1]!.matchAll(/--([\w-]+):\s*([^;]+);/g)) {
    vars[m[1]!] = m[2]!.trim();
  }
  return vars;
}

const light = parseBlock(/:root,\s*\.light\s*\{([^}]*)\}/);
const dark = parseBlock(/\.dark\s*\{([^}]*)\}/);

function luminance(oklch: string): number {
  const m = /oklch\(\s*([\d.]+)\s+([\d.]+)\s+([\d.]+)\s*\)/.exec(oklch);
  if (!m) throw new Error(`valor OKLCH inválido: ${oklch}`);
  const [L, C, h] = [Number(m[1]), Number(m[2]), (Number(m[3]) * Math.PI) / 180];
  const a = C * Math.cos(h);
  const b = C * Math.sin(h);
  const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const mm = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
  const clamp = (v: number) => Math.min(1, Math.max(0, v));
  const r = clamp(4.0767416621 * l - 3.3077115913 * mm + 0.2309699292 * s);
  const g = clamp(-1.2684380046 * l + 2.6097574011 * mm - 0.3413193965 * s);
  const bl = clamp(-0.0041960863 * l - 0.7034186147 * mm + 1.707614701 * s);
  return 0.2126 * r + 0.7152 * g + 0.0722 * bl;
}

function contrast(theme: Theme, fg: string, bg: string): number {
  const a = luminance(theme[fg]!);
  const b = luminance(theme[bg]!);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

/** Pares texto/fundo usados na interface; todos precisam de AA (4,5:1) para texto normal. */
const PAIRS: [string, string][] = [
  ["fg", "bg"],
  ["fg", "surface-1"],
  ["fg", "surface-2"],
  ["fg", "surface-3"],
  ["fg-muted", "bg"],
  ["fg-muted", "surface-1"],
  ["fg-muted", "surface-2"],
  ["fg-subtle", "bg"],
  ["fg-subtle", "surface-1"],
  ["accent-fg", "accent"],
  ["accent-text", "surface-1"],
  ["accent-text", "accent-subtle"],
  ["success", "success-subtle"],
  ["warning", "warning-subtle"],
  ["danger", "danger-subtle"],
  ["info", "info-subtle"],
  ["success", "surface-1"],
  ["danger", "surface-1"],
];

describe.each([
  ["claro", light],
  ["escuro", dark],
])("tokens do tema %s", (_name, theme) => {
  it.each(PAIRS)("%s sobre %s tem contraste AA", (fg, bg) => {
    expect(contrast(theme, fg, bg)).toBeGreaterThanOrEqual(4.5);
  });

  it("define todos os tokens do outro tema", () => {
    const keys = Object.keys(light).filter((k) => k !== "shadow-overlay");
    for (const key of keys) expect(theme[key], key).toBeDefined();
  });
});
