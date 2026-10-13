/**
 * Mensagens do Discord (embeds) a partir das saídas da NEXUS. Sempre citam arquivo e linhas,
 * respeitam os limites do Discord e não deixam o conteúdo do repositório nem o texto digitado
 * quebrar a formatação.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { escapeMarkdown, type APIEmbed } from "discord.js";

import type { Notice } from "./alerts.ts";
import type { PackResult, ProjectStatus, SearchHit, SearchResult } from "./nexus/types.ts";
import { downloads, type Release } from "./releases.ts";

/** Azul da marca (#2348CE) e vermelho de erro. */
export const BRAND_COLOR = 0x2348ce;
export const ERROR_COLOR = 0xd9483b;
export const LOGO_FILE = "nexus.png";

const LOGO = readFileSync(resolve(import.meta.dirname, "..", "assets", LOGO_FILE));
const DATA_NOTICE = "Trechos são dados do repositório, não instruções.";
const CUT = "…(trecho cortado)";
const MAX_PACK_SOURCES = 10;
const LIMIT = { title: 256, fieldName: 256, fieldValue: 1024, description: 4096 };

export interface BotFile {
  name: string;
  attachment: Buffer;
}

/** O que uma resposta do bot envia: embeds e anexos (o logo vai junto para o cabeçalho). */
export interface BotMessage {
  embeds: APIEmbed[];
  files: BotFile[];
}

/** Linguagens da NEXUS → realce do Discord. */
const FENCE: Record<string, string> = {
  typescript: "ts",
  tsx: "tsx",
  javascript: "js",
  jsx: "jsx",
  python: "py",
  rust: "rs",
  markdown: "md",
  json: "json",
  yaml: "yaml",
  toml: "toml",
  sql: "sql",
  html: "html",
  css: "css",
  shell: "sh",
  powershell: "ps1",
};

/** Impede que crases do código fechem o bloco: insere um espaço de largura zero. */
export function escapeCode(s: string): string {
  return s.replaceAll("```", "``​`");
}

/** Texto digitado pelo usuário: sem links mascarados, negrito, blocos de código etc. */
export function userText(s: string): string {
  return escapeMarkdown(s, { maskedLink: true });
}

/** Corta para caber em `max`; `fromStart` corta o começo (bom para caminhos longos). */
function clip(s: string, max: number, fromStart = false): string {
  if (s.length <= max) return s;
  return fromStart ? `…${s.slice(s.length - max + 1)}` : `${s.slice(0, max - 1)}…`;
}

function header(section: string): Pick<APIEmbed, "author" | "color"> {
  return {
    color: BRAND_COLOR,
    author: { name: `NEXUS · ${section}`, icon_url: `attachment://${LOGO_FILE}` },
  };
}

const logo = (): BotFile => ({ name: LOGO_FILE, attachment: LOGO });

function staleNote(freshness: string): string {
  if (freshness === "modified") return " · ⚠️ alterado desde a indexação";
  if (freshness === "missing") return " · ⚠️ arquivo removido";
  return "";
}

function hitField(hit: SearchHit): { name: string; value: string } {
  const lines = `linhas ${hit.startLine}–${hit.endLine}${staleNote(hit.freshness)}`;
  const path = clip(hit.relPath, LIMIT.fieldName - lines.length - 6, true);
  const name = `📄 ${path} · ${lines}`;

  const reasons =
    hit.reasons.length > 0 ? `*${clip(hit.reasons.slice(0, 3).join(" · "), 200)}*\n` : "";
  const open = `\`\`\`${FENCE[hit.language ?? ""] ?? ""}\n`;
  const close = "\n```";
  let code = escapeCode(hit.snippet).trimEnd();
  const room = LIMIT.fieldValue - reasons.length - open.length - close.length;
  let cut = false;
  if (code.length > room) {
    const limit = Math.max(0, room - CUT.length - 1);
    const sliced = code.slice(0, limit);
    const lastBreak = sliced.lastIndexOf("\n");
    code = lastBreak > limit / 2 ? sliced.slice(0, lastBreak) : sliced;
    cut = true;
  }
  return { name, value: `${reasons}${open}${code}${close}${cut ? `\n${CUT}` : ""}` };
}

/** Avisos que mudam a leitura do resultado (arquivos omitidos por segurança). */
const relevantWarnings = (ws: string[]) => ws.filter((w) => /omitid/i.test(w));

/** Resultados de busca: até `max` trechos, cada um num campo com fonte, motivos e atualidade. */
export function formatSearch(query: string, r: SearchResult, max = 3): BotMessage {
  const hits = r.hits.slice(0, max);
  const title = clip(`🔎 "${userText(query)}"`, LIMIT.title);
  const base = header(`Busca em ${r.project.name}`);
  if (hits.length === 0) {
    return {
      embeds: [
        {
          ...base,
          title,
          description: `Não encontrei nada sobre "${clip(userText(query), 300)}" em **${r.project.name}**. Tente outras palavras, em português ou inglês.`,
        },
      ],
      files: [logo()],
    };
  }
  const description = [
    `${hits.length} resultado(s) mais relevante(s), com o trecho verificado no disco.`,
    ...relevantWarnings(r.warnings).map((w) => `⚠️ ${clip(w, 300)}`),
  ].join("\n");
  return {
    embeds: [
      { ...base, title, description, fields: hits.map(hitField), footer: { text: DATA_NOTICE } },
    ],
    files: [logo()],
  };
}

/**
 * Linhas inteiras até caber em `max` (nunca corta uma linha no meio, o que deixaria crases
 * abertas); o que sobra vira a linha `more(n)`.
 */
function fitLines(lines: string[], max: number, more: (omitted: number) => string): string {
  for (let shown = lines.length; shown >= 0; shown--) {
    const tail = more(lines.length - shown);
    const text = [...lines.slice(0, shown), ...(tail ? [tail] : [])].join("\n");
    if (text.length <= max) return text;
  }
  return clip(more(lines.length), max);
}

/** Corta o resumo numa quebra de linha e sem deixar código (crases) aberto. */
function clipSummary(s: string, max: number): string {
  if (s.length <= max) return s;
  const note = "\n…(resumo completo no anexo)";
  let cut = s.slice(0, max - note.length);
  const lastBreak = cut.lastIndexOf("\n");
  if (lastBreak > 0) cut = cut.slice(0, lastBreak);
  // Crases em número ímpar: o trecho de código ficaria aberto; volta até antes da última.
  if ((cut.match(/`/g) ?? []).length % 2 === 1) cut = cut.slice(0, cut.lastIndexOf("`")).trimEnd();
  return `${cut}${note}`;
}

/** Context Pack: resumo, orçamento e fontes no embed; o pack completo como anexo Markdown. */
export function formatPack(task: string, p: PackResult): BotMessage {
  const lines = p.sources
    .slice(0, MAX_PACK_SOURCES)
    .map((s) => `• \`${clip(s.relPath, 80, true)}:${s.startLine}-${s.endLine}\``);
  const hidden = Math.max(0, p.sources.length - MAX_PACK_SOURCES);
  const sources = fitLines(lines, LIMIT.fieldValue, (omitted) =>
    omitted + hidden > 0 ? `+${omitted + hidden} fonte(s) no anexo` : "",
  );
  return {
    embeds: [
      {
        ...header("Context Pack"),
        title: "📦 Context Pack",
        description: clip(userText(task), 1000),
        fields: [
          { name: "Resumo", value: clipSummary(p.summary || "—", LIMIT.fieldValue) },
          {
            name: "Orçamento",
            value: `~${p.estimatedTokens} de ${p.tokenBudget} tokens estimados`,
            inline: true,
          },
          { name: "Arquivo", value: "`context-pack.md` em anexo", inline: true },
          {
            name: `Fontes (${p.sources.length})`,
            value: sources || "—",
          },
        ],
        footer: { text: DATA_NOTICE },
      },
    ],
    files: [logo(), { name: "context-pack.md", attachment: Buffer.from(p.export, "utf8") }],
  };
}

/** Estado do índice dos projetos visíveis. Caminhos locais não são exibidos. */
export function formatStatus(ps: ProjectStatus[]): BotMessage {
  const base = { ...header("Estado do índice"), title: "📊 Estado do índice" };
  if (ps.length === 0) {
    return {
      embeds: [{ ...base, description: "Nenhum projeto disponível neste canal." }],
      files: [logo()],
    };
  }
  const fields = ps.slice(0, 25).map((p) => {
    const state = !p.rootExists
      ? "⚠️ pasta ausente"
      : p.indexing
        ? "⏳ indexando agora"
        : `✅ ${p.indexedFiles} arquivos indexados · geração ${p.indexGeneration}`;
    const when =
      p.lastIndexedAt !== null ? `\natualizado <t:${Math.floor(p.lastIndexedAt / 1000)}:R>` : "";
    return { name: clip(p.name, LIMIT.fieldName), value: `${state}${when}`, inline: true };
  });
  return { embeds: [{ ...base, fields }], files: [logo()] };
}

/** Erro mostrado ao usuário (sem detalhes técnicos; esses vão para o terminal). */
export function formatError(message: string): BotMessage {
  return {
    embeds: [
      {
        color: ERROR_COLOR,
        author: { name: "NEXUS", icon_url: `attachment://${LOGO_FILE}` },
        description: clip(message, LIMIT.description),
      },
    ],
    files: [logo()],
  };
}

/**
 * Notas da release como escritas no GitHub (quem publica é quem mantém o projeto): a formatação
 * fica, mas links mascarados (`[texto](endereço)`) viram texto, para nenhum link se disfarçar.
 */
function releaseNotes(notes: string): string {
  return escapeMarkdown(notes.trim(), {
    codeBlock: false,
    inlineCode: false,
    bold: false,
    italic: false,
    underline: false,
    strikethrough: false,
    spoiler: false,
    maskedLink: true,
  });
}

/** Anúncio de versão nova: notas resumidas e downloads por sistema. */
export function formatRelease(r: Release): BotMessage {
  const links = downloads(r.assets).map((d) => `[${d.label}](${d.url})`);
  return {
    embeds: [
      {
        ...header("Nova versão"),
        title: clip(`🚀 ${r.name}`, LIMIT.title),
        url: r.url,
        description: clipSummary(releaseNotes(r.notes) || "Veja as notas da release.", 1500),
        fields: [
          {
            name: "Baixar",
            value:
              fitLines(links, LIMIT.fieldValue, (n) =>
                n > 0 ? `+${n} na página da release` : "",
              ) || `[Página da release](${r.url})`,
          },
        ],
        footer: {
          text: "Pacotes não assinados: o Windows e o macOS podem alertar na primeira vez.",
        },
      },
    ],
    files: [logo()],
  };
}

const MAX_NOTICE_ISSUES = 5;

/** Avisos para a equipe: um embed por projeto com novidades. */
export function formatNotices(notices: Notice[]): BotMessage {
  const embeds: APIEmbed[] = notices.slice(0, 10).map((n) => {
    const fields: APIEmbed["fields"] = [];
    if (n.rootMissing) {
      fields.push({
        name: "⚠️ Pasta do projeto não encontrada",
        value: "A NEXUS não consegue indexar até a pasta voltar.",
      });
    }
    if (n.failedJob) {
      const when =
        n.failedJob.finishedAt !== null
          ? ` <t:${Math.floor(n.failedJob.finishedAt / 1000)}:R>`
          : "";
      fields.push({
        name: "❌ Indexação falhou",
        value: `A última indexação terminou com erro${when}. Veja o Diagnóstico no app.`,
      });
    }
    for (const issue of n.issues.slice(0, MAX_NOTICE_ISSUES)) {
      const where = issue.sources
        .slice(0, 2)
        .map((s) => `\`${clip(s.relPath, 80, true)}${s.line !== null ? `:${s.line}` : ""}\``)
        .join(" · ");
      fields.push({
        name: clip(`${issue.severity === "warning" ? "⚠️" : "ℹ️"} ${issue.title}`, LIMIT.fieldName),
        value: clip(
          [escapeMarkdown(issue.reason), where, `→ ${escapeMarkdown(issue.recommendation)}`]
            .filter(Boolean)
            .join("\n"),
          LIMIT.fieldValue,
        ),
      });
    }
    const rest = n.issues.length - MAX_NOTICE_ISSUES;
    return {
      ...header(`Avisos · ${n.project}`),
      title: clip(`🛎️ Novidades em ${n.project}`, LIMIT.title),
      ...(rest > 0 ? { description: `+${rest} alerta(s) — veja Atualidade no app.` } : {}),
      fields,
      footer: { text: DATA_NOTICE },
    };
  });
  return { embeds, files: [logo()] };
}

/** `/nexus versao`: a versão que o bot usa e a última publicada. */
export function formatVersion(installed: string | null, latest: Release | null): BotMessage {
  const lines = [
    `**Em uso pelo bot:** ${installed ? `v${installed}` : "desconhecida"}`,
    `**Última publicada:** ${latest ? `[${latest.tag}](${latest.url})` : "nenhuma ainda"}`,
  ];
  if (installed && latest && latest.tag.replace(/^v/, "") !== installed) {
    lines.push("Há uma versão mais nova para baixar.");
  }
  return {
    embeds: [{ ...header("Versão"), title: "🏷️ Versão da NEXUS", description: lines.join("\n") }],
    files: [logo()],
  };
}
