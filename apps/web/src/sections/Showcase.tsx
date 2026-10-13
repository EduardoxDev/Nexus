import { BookOpenCheck, Cable, FolderGit2, PackageOpen, ScanSearch } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useState } from "react";

import { Reveal } from "@/components/fx";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

const SCREENS = [
  {
    id: "busca",
    icon: ScanSearch,
    label: "Busca",
    src: "/screens/busca-com-previa.webp",
    alt: "Busca com motivos de relevância e prévia do arquivo",
    caption: "Motivos de cada resultado, prévia verificada e o grafo de quem importa quem.",
  },
  {
    id: "pack",
    icon: PackageOpen,
    label: "Context Pack",
    src: "/screens/context-pack.webp",
    alt: "Context Pack com arquivos, testes e decisões",
    caption: "Um pacote de contexto para uma tarefa, pronto para colar ou para o agente buscar.",
  },
  {
    id: "memoria",
    icon: BookOpenCheck,
    label: "Memória",
    src: "/screens/memoria.webp",
    alt: "Memória do projeto com histórico de revisões",
    caption: "Decisões confirmadas por você, com referências verificáveis e histórico.",
  },
  {
    id: "projeto",
    icon: FolderGit2,
    label: "Projeto",
    src: "/screens/projeto.webp",
    alt: "Detalhes do projeto indexado",
    caption: "Linguagens, estrutura, manifestos e o estado do índice — tudo real.",
  },
  {
    id: "integracoes",
    icon: Cable,
    label: "Integrações",
    src: "/screens/integracoes.webp",
    alt: "Integrações com o comando do Claude Code",
    caption: "O comando para o Claude Code e a configuração MCP, com os caminhos certos.",
  },
];

export function Showcase() {
  const [tab, setTab] = useState(SCREENS[0]!.id);
  return (
    <section className="relative py-16 sm:py-20">
      <div className="mx-auto max-w-6xl px-5">
        <Reveal className="mx-auto max-w-3xl text-center">
          <p className="text-sm font-semibold tracking-[0.16em] text-primary uppercase">
            Por dentro
          </p>
          <h2 className="mt-3 font-display text-4xl font-bold sm:text-5xl">
            Um app que mostra o porquê.
          </h2>
          <p className="mt-4 text-lg text-muted-foreground">
            Telas reais do app — nada de mockup. Sem métricas inventadas, sem progresso falso.
          </p>
        </Reveal>

        <Reveal className="mt-12">
          <Tabs value={tab} onValueChange={setTab} className="flex flex-col items-center">
            <TabsList aria-label="Telas do app">
              {SCREENS.map((s) => (
                <TabsTrigger key={s.id} value={s.id}>
                  <s.icon aria-hidden="true" />
                  {s.label}
                </TabsTrigger>
              ))}
            </TabsList>
            {SCREENS.map((s) => (
              <TabsContent key={s.id} value={s.id} className="mt-8 w-full outline-none">
                <AnimatePresence mode="wait">
                  {tab === s.id && (
                    <motion.figure
                      key={s.id}
                      initial={{ opacity: 0, y: 16, scale: 0.985 }}
                      animate={{ opacity: 1, y: 0, scale: 1 }}
                      exit={{ opacity: 0 }}
                      transition={{ duration: 0.45, ease: [0.22, 1, 0.36, 1] }}
                    >
                      <div className="overflow-hidden rounded-2xl border border-border bg-card p-1.5 shadow-[0_30px_80px_-30px_oklch(0.4_0.15_266/0.45)]">
                        <img
                          src={s.src}
                          alt={s.alt}
                          width={1440}
                          height={900}
                          loading="lazy"
                          className="w-full rounded-xl"
                        />
                      </div>
                      <figcaption className="mt-4 text-center text-sm text-muted-foreground">
                        {s.caption}
                      </figcaption>
                    </motion.figure>
                  )}
                </AnimatePresence>
              </TabsContent>
            ))}
          </Tabs>
        </Reveal>
      </div>
    </section>
  );
}
