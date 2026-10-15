import { Markdown, parseBlocks, visibleText } from "@/lib/markdown";
import { DOCS, pageHref, type DocRoute } from "@/routes";
import { site } from "@/site";

import { Footer } from "./Footer";
import { Nav } from "./Nav";

/** Página de documentação: menu das páginas ao lado e o Markdown renderizado. */
export function DocPage({ doc }: { doc: DocRoute }) {
  const blocks = parseBlocks(doc.markdown);
  const sections = blocks.filter((b) => b.t === "h" && b.level === 2);
  const ctx = { source: doc.source, base: import.meta.env.BASE_URL, repo: site.githubUrl };
  return (
    <>
      <Nav home={false} />
      <main id="conteudo" className="mx-auto max-w-6xl px-5 pt-12 pb-24">
        <div className="grid gap-12 md:grid-cols-[13rem_minmax(0,1fr)]">
          <aside className="md:sticky md:top-24 md:self-start">
            <nav aria-label="Documentação">
              <p className="mb-3 text-xs font-medium tracking-wider text-muted-foreground uppercase">
                Documentação
              </p>
              <ul className="space-y-1 text-sm">
                {DOCS.map((d) => (
                  <li key={d.path}>
                    <a
                      href={pageHref(d.path)}
                      aria-current={d.path === doc.path ? "page" : undefined}
                      className={`block rounded-lg px-3 py-1.5 transition-colors ${
                        d.path === doc.path
                          ? "bg-muted text-foreground"
                          : "text-muted-foreground hover:text-foreground"
                      }`}
                    >
                      {d.label}
                    </a>
                  </li>
                ))}
              </ul>
              {sections.length > 1 && (
                <>
                  <p className="mt-8 mb-3 text-xs font-medium tracking-wider text-muted-foreground uppercase">
                    Nesta página
                  </p>
                  <ul className="space-y-1 text-sm">
                    {sections.map(
                      (s) =>
                        s.t === "h" && (
                          <li key={s.id}>
                            <a
                              href={`#${s.id}`}
                              className="block px-3 py-1 text-muted-foreground hover:text-foreground"
                            >
                              {visibleText(s.text)}
                            </a>
                          </li>
                        ),
                    )}
                  </ul>
                </>
              )}
            </nav>
          </aside>
          <article className="min-w-0 max-w-3xl">
            <Markdown blocks={blocks} ctx={ctx} />
          </article>
        </div>
      </main>
      <Footer />
    </>
  );
}
