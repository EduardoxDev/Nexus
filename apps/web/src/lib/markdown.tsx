/**
 * Markdown → React, só o que a documentação usa: títulos, parágrafos, listas (com um nível de
 * sublista), tabelas, blocos de código, citações, `código`, **negrito**, _itálico_ e links.
 * Sem HTML cru: tudo vira elemento React (o texto é escapado pelo React). Links relativos do
 * repositório viram links para o GitHub; links do site respeitam a base (/ ou /Nexus/).
 */
import { Fragment, type ReactNode } from "react";

export type Block =
  | { t: "h"; level: 1 | 2 | 3; text: string; id: string }
  | { t: "p"; text: string }
  | { t: "list"; ordered: boolean; items: { text: string; children: string[] }[] }
  | { t: "code"; lang: string; text: string }
  | { t: "table"; head: string[]; rows: string[][] }
  | { t: "quote"; text: string };

/** Âncora de um título, como no GitHub (sem acento, minúsculas, hífens). */
export function slug(text: string): string {
  return text
    .replace(/[`*_[\]()]/gu, "")
    .normalize("NFKD")
    .replace(/[̀-ͯ]/gu, "")
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/gu, "-")
    .replace(/^-+|-+$/gu, "");
}

/** Texto como aparece na tela: links viram o rótulo, sem marcas de ênfase ou código. */
export function visibleText(md: string): string {
  return md.replace(/\[([^\]]+)\]\([^)\s]+\)/gu, "$1").replace(/[`*]/gu, "");
}

const LIST_ITEM = /^(\s*)([-*]|\d+\.)\s+(.*)$/u;
const cells = (line: string) =>
  line
    .trim()
    .replace(/^\||\|$/gu, "")
    .split(/(?<!\\)\|/u)
    .map((c) => c.trim().replaceAll("\\|", "|"));

export function parseBlocks(md: string): Block[] {
  const lines = md.replace(/\r\n?/gu, "\n").split("\n");
  const out: Block[] = [];
  const ids = new Map<string, number>();
  let i = 0;
  while (i < lines.length) {
    const line = lines[i] ?? "";
    if (!line.trim()) {
      i++;
      continue;
    }
    const fence = /^```(\w*)\s*$/u.exec(line);
    if (fence) {
      const body: string[] = [];
      i++;
      while (i < lines.length && !/^```\s*$/u.test(lines[i] ?? "")) body.push(lines[i++] ?? "");
      i++;
      out.push({ t: "code", lang: fence[1] ?? "", text: body.join("\n") });
      continue;
    }
    const h = /^(#{1,3})\s+(.*)$/u.exec(line);
    if (h) {
      const text = (h[2] ?? "").trim();
      // A âncora usa o texto visível: num link, o rótulo (não o destino).
      const base = slug(visibleText(text));
      const n = ids.get(base) ?? 0;
      ids.set(base, n + 1);
      out.push({
        t: "h",
        level: (h[1] ?? "#").length as 1 | 2 | 3,
        text,
        id: n ? `${base}-${n}` : base,
      });
      i++;
      continue;
    }
    if (line.trimStart().startsWith("|") && /^\s*\|?\s*:?-+:?\s*(\||$)/u.test(lines[i + 1] ?? "")) {
      const head = cells(line);
      i += 2;
      const rows: string[][] = [];
      while (i < lines.length && (lines[i] ?? "").trimStart().startsWith("|"))
        rows.push(cells(lines[i++] ?? ""));
      out.push({ t: "table", head, rows });
      continue;
    }
    if (line.startsWith(">")) {
      const body: string[] = [];
      while (i < lines.length && (lines[i] ?? "").startsWith(">"))
        body.push((lines[i++] ?? "").replace(/^>\s?/u, ""));
      out.push({ t: "quote", text: body.join(" ") });
      continue;
    }
    const first = LIST_ITEM.exec(line);
    if (first && (first[1] ?? "").length === 0) {
      const ordered = /\d/u.test(first[2] ?? "");
      const items: { text: string; children: string[] }[] = [];
      // Coluna onde começa o texto da sublista atual: continuação recuada até ela é da sublista;
      // menos recuada, é do item pai.
      let childColumn = Infinity;
      while (i < lines.length) {
        const l = lines[i] ?? "";
        const m = LIST_ITEM.exec(l);
        if (m && (m[1] ?? "").length === 0) {
          items.push({ text: m[3] ?? "", children: [] });
          childColumn = Infinity;
        } else if (m && items.length > 0) {
          items.at(-1)?.children.push(m[3] ?? "");
          childColumn = m[0].length - (m[3] ?? "").length;
        } else if (l.trim() && /^\s+/u.test(l) && items.length > 0) {
          // Continuação da linha anterior (do item ou da sublista).
          const last = items.at(-1);
          const lead = l.length - l.trimStart().length;
          if (last && last.children.length > 0 && lead >= childColumn) {
            last.children[last.children.length - 1] += ` ${l.trim()}`;
          } else if (last) {
            last.text += ` ${l.trim()}`;
          }
        } else {
          break;
        }
        i++;
      }
      out.push({ t: "list", ordered, items });
      continue;
    }
    // A primeira linha sempre entra: o laço avança mesmo numa linha que nenhum bloco aceitou
    // (ex.: "| x" sem a linha de separação de tabela).
    const para: string[] = [line.trim()];
    i++;
    while (i < lines.length) {
      const l = lines[i] ?? "";
      if (!l.trim() || /^(#{1,3}\s|```|>|\s*\|)/u.test(l) || (LIST_ITEM.test(l) && !/^\s/u.test(l)))
        break;
      para.push(l.trim());
      i++;
    }
    out.push({ t: "p", text: para.join(" ") });
  }
  return out;
}

export interface LinkContext {
  /** Arquivo de origem no repositório (para resolver links relativos), ex.: `docs/privacy.md`. */
  source: string;
  /** Base do site (`/` ou `/Nexus/`). */
  base: string;
  /** Repositório, ex.: `https://github.com/dono/repo`. */
  repo: string;
}

/** Destino seguro de um link: http(s), mailto, âncora, página do site ou arquivo do repositório. */
export function resolveHref(href: string, ctx: LinkContext): string | null {
  if (/^(https?:|mailto:)/iu.test(href)) return href;
  if (href.startsWith("#")) return href;
  if (/^[a-z][a-z0-9+.-]*:/iu.test(href)) return null; // javascript:, data: e afins
  if (href.startsWith("/")) return ctx.base + href.slice(1);
  const dir = ctx.source.split("/").slice(0, -1);
  const [path = "", hash] = href.split("#");
  const parts = [...dir];
  for (const seg of path.split("/")) {
    if (seg === "..") parts.pop();
    else if (seg && seg !== ".") parts.push(seg);
  }
  return `${ctx.repo}/blob/main/${parts.join("/")}${hash ? `#${hash}` : ""}`;
}

const INLINE =
  /(`+)([\s\S]*?)\1|\*\*([\s\S]+?)\*\*|(?<![\w*])_([^_]+)_(?!\w)|\[([^\]]+)\]\(([^)\s]+)\)/gu;

export function inline(text: string, ctx: LinkContext): ReactNode[] {
  const out: ReactNode[] = [];
  let last = 0;
  let k = 0;
  for (const m of text.matchAll(INLINE)) {
    const at = m.index ?? 0;
    if (at > last) out.push(text.slice(last, at));
    last = at + m[0].length;
    if (m[1]) {
      out.push(
        <code
          key={k++}
          className="font-codigo rounded-md bg-muted px-1.5 py-0.5 text-[0.9em] text-foreground"
        >
          {(m[2] ?? "").trim()}
        </code>,
      );
    } else if (m[3] !== undefined) {
      out.push(
        <strong key={k++} className="font-semibold text-foreground">
          {inline(m[3], ctx)}
        </strong>,
      );
    } else if (m[4] !== undefined) {
      out.push(<em key={k++}>{inline(m[4], ctx)}</em>);
    } else if (m[5] !== undefined) {
      const href = resolveHref(m[6] ?? "", ctx);
      const label = inline(m[5], ctx);
      out.push(
        href ? (
          <a
            key={k++}
            href={href}
            {...(/^https?:/u.test(href) ? { rel: "noopener" } : {})}
            className="text-foreground underline decoration-primary/60 underline-offset-4 hover:decoration-primary"
          >
            {label}
          </a>
        ) : (
          <Fragment key={k++}>{label}</Fragment>
        ),
      );
    }
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

const HEADING = {
  1: "font-titulo text-[clamp(2rem,4vw,2.75rem)] leading-[1.1] text-foreground",
  2: "mt-14 scroll-mt-24 font-titulo text-2xl text-foreground",
  3: "mt-8 scroll-mt-24 font-titulo text-lg text-foreground",
} as const;

export function Markdown({ blocks, ctx }: { blocks: Block[]; ctx: LinkContext }) {
  return (
    <>
      {blocks.map((b, n) => {
        switch (b.t) {
          case "h": {
            const Tag = `h${b.level}` as const;
            return (
              <Tag key={n} id={b.id} className={HEADING[b.level]}>
                {inline(b.text, ctx)}
              </Tag>
            );
          }
          case "p":
            return (
              <p key={n} className="mt-4 leading-relaxed text-muted-foreground">
                {inline(b.text, ctx)}
              </p>
            );
          case "quote":
            return (
              <blockquote
                key={n}
                className="mt-4 border-l-2 border-primary/60 pl-4 text-muted-foreground"
              >
                {inline(b.text, ctx)}
              </blockquote>
            );
          case "code":
            return (
              <pre
                key={n}
                className="font-codigo mt-4 overflow-x-auto rounded-xl border border-white/10 bg-black/40 p-4 text-[14px] leading-relaxed text-[#dbe4ff]"
              >
                <code>{b.text}</code>
              </pre>
            );
          case "list": {
            const List = b.ordered ? "ol" : "ul";
            return (
              <List
                key={n}
                className={`mt-4 space-y-2 pl-6 text-muted-foreground ${b.ordered ? "list-decimal" : "list-disc"} marker:text-primary/70`}
              >
                {b.items.map((it, j) => (
                  <li key={j} className="leading-relaxed">
                    {inline(it.text, ctx)}
                    {it.children.length > 0 && (
                      <ul className="mt-2 list-[circle] space-y-1.5 pl-6">
                        {it.children.map((c, x) => (
                          <li key={x}>{inline(c, ctx)}</li>
                        ))}
                      </ul>
                    )}
                  </li>
                ))}
              </List>
            );
          }
          case "table":
            return (
              <div key={n} className="mt-6 overflow-x-auto rounded-xl border">
                <table className="w-full text-left text-sm">
                  <thead className="bg-muted/50 text-foreground">
                    <tr>
                      {b.head.map((c, j) => (
                        <th key={j} scope="col" className="px-4 py-2.5 font-medium">
                          {inline(c, ctx)}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody className="text-muted-foreground">
                    {b.rows.map((r, j) => (
                      <tr key={j} className="border-t align-top">
                        {r.map((c, x) => (
                          <td key={x} className="px-4 py-2.5">
                            {inline(c, ctx)}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            );
        }
      })}
    </>
  );
}
