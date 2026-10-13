import {
  BookOpenCheck,
  Cable,
  Code2,
  PackageOpen,
  Radar,
  ScanSearch,
  type LucideIcon,
} from "lucide-react";
import type { ReactNode } from "react";

import { Reveal, SpotlightCard } from "@/components/fx";
import { cn } from "@/lib/cn";

interface Feature {
  icon: LucideIcon;
  title: string;
  body: string;
  className?: string;
  visual?: ReactNode;
}

function Shot({ src, alt, position = "top" }: { src: string; alt: string; position?: string }) {
  return (
    <div className="relative mt-6 -mr-7 -mb-7 overflow-hidden rounded-tl-xl border-t border-l border-border">
      <img
        src={src}
        alt={alt}
        loading="lazy"
        width={1440}
        height={900}
        className="w-full transition-transform duration-700 ease-out group-hover:scale-[1.03]"
        style={{ objectPosition: position }}
      />
      <div className="pointer-events-none absolute inset-0 bg-gradient-to-t from-card via-transparent to-transparent" />
    </div>
  );
}

const TOOLS = [
  "search_project",
  "get_file_context",
  "get_symbol_context",
  "get_related_files",
  "build_context_pack",
  "get_project_memory",
  "get_project_decisions",
  "get_stale_context",
];

const FEATURES: Feature[] = [
  {
    icon: ScanSearch,
    title: "Busca com evidências",
    body: "Em português ou inglês. Cada resultado traz arquivo, linhas, o motivo de ter aparecido e se o trecho ainda bate com o disco.",
    className: "md:col-span-2",
    visual: (
      <Shot src="/screens/busca-com-previa.webp" alt="Busca com motivos e prévia verificada" />
    ),
  },
  {
    icon: Cable,
    title: "Feita para agentes (MCP)",
    body: "Servidor MCP local com 13 ferramentas. O Claude Code consulta a NEXUS sozinho.",
    visual: (
      <ul className="mt-6 space-y-1.5 font-mono text-[12.5px]">
        {TOOLS.map((t, i) => (
          <li
            key={t}
            className="flex items-center gap-2 text-muted-foreground"
            style={{ opacity: 1 - i * 0.09 }}
          >
            <span className="size-1.5 rounded-full bg-primary" />
            {t}
          </li>
        ))}
      </ul>
    ),
  },
  {
    icon: PackageOpen,
    title: "Context Packs",
    body: "Para uma tarefa: os arquivos certos, os testes, as decisões e as convenções — dentro de um orçamento de tokens, com cada fonte conferida.",
    visual: <Shot src="/screens/context-pack.webp" alt="Context Pack compilado para uma tarefa" />,
  },
  {
    icon: BookOpenCheck,
    title: "Memória do projeto",
    body: "Decisões, convenções e lições com histórico de revisões. Só o que você confirma chega aos agentes; ADRs entram como propostas.",
    visual: <Shot src="/screens/memoria.webp" alt="Memória do projeto com histórico" />,
  },
  {
    icon: Radar,
    title: "Atualidade",
    body: "Avisa quando a documentação cita o que sumiu, quando uma decisão mudou na origem e quando duas memórias se contradizem.",
  },
  {
    icon: Code2,
    title: "Entende o código",
    body: "Declarações e imports de TypeScript, JavaScript, Python e Rust viram um grafo de dependências com a linha de cada import como prova.",
    className: "md:col-span-3",
    visual: (
      <div className="mt-6 flex flex-wrap gap-2">
        {[
          "TypeScript",
          "JavaScript",
          "Python",
          "Rust",
          "Markdown",
          "+ busca em qualquer texto",
        ].map((l) => (
          <span
            key={l}
            className="rounded-md border border-border bg-muted/60 px-2.5 py-1 font-mono text-xs text-muted-foreground"
          >
            {l}
          </span>
        ))}
      </div>
    ),
  },
];

export function Features() {
  return (
    <section id="recursos" className="relative scroll-mt-20 py-16 sm:py-20">
      <div className="mx-auto max-w-6xl px-5">
        <Reveal className="mx-auto max-w-3xl text-center">
          <p className="text-sm font-semibold tracking-[0.16em] text-primary uppercase">Recursos</p>
          <h2 className="mt-3 font-display text-4xl font-bold sm:text-5xl">
            Tudo o que um agente precisa saber.
            <br />
            <span className="text-muted-foreground">Nada que ele não deva.</span>
          </h2>
        </Reveal>

        <div className="mt-14 grid gap-4 md:grid-cols-3">
          {FEATURES.map((f, i) => (
            <Reveal key={f.title} delay={(i % 3) * 0.08} className={cn("flex", f.className)}>
              <SpotlightCard className="flex w-full flex-col p-7">
                <span className="flex size-10 items-center justify-center rounded-xl border border-primary/20 bg-accent text-accent-foreground">
                  <f.icon className="size-5" aria-hidden="true" />
                </span>
                <h3 className="mt-5 font-display text-xl font-semibold">{f.title}</h3>
                <p className="mt-2 text-[15px] leading-relaxed text-muted-foreground">{f.body}</p>
                {f.visual}
              </SpotlightCard>
            </Reveal>
          ))}
        </div>
      </div>
    </section>
  );
}
