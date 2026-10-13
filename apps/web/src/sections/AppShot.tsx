import { BookOpenCheck, Cable, LayoutDashboard, PackageOpen, ScanSearch } from "lucide-react";
import { useState } from "react";

import { Bento, Shot } from "@/components/Bento";
import { SectionHeader } from "@/components/SectionHeader";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { asset } from "@/lib/asset";

const SCREENS = [
  {
    id: "visao",
    label: "Visão geral",
    icon: LayoutDashboard,
    src: asset("/screens/visao-geral-escuro.webp"),
    alt: "Visão geral com indicadores do projeto",
    caption: "Indicadores reais do projeto, operações recentes e o estado do sistema.",
  },
  {
    id: "busca",
    label: "Busca",
    icon: ScanSearch,
    src: asset("/screens/busca-com-previa.webp"),
    alt: "Busca com motivos e prévia do arquivo",
    caption: "Cada resultado com o motivo de ter aparecido e a prévia conferida no disco.",
  },
  {
    id: "pack",
    label: "Context Pack",
    icon: PackageOpen,
    src: asset("/screens/context-pack.webp"),
    alt: "Context Pack para uma tarefa",
    caption: "Arquivos, testes e decisões de uma tarefa, prontos para o agente.",
  },
  {
    id: "memoria",
    label: "Memória",
    icon: BookOpenCheck,
    src: asset("/screens/memoria.webp"),
    alt: "Memória do projeto com histórico",
    caption: "Decisões confirmadas por você, com histórico de revisões.",
  },
  {
    id: "integracoes",
    label: "Integrações",
    icon: Cable,
    src: asset("/screens/integracoes.webp"),
    alt: "Integrações com o comando do Claude Code",
    caption: "O comando do Claude Code e a configuração MCP, com os caminhos certos.",
  },
];

/** As telas reais do app: abas do shadcn e a captura num card do bento. */
export function AppShot() {
  const [tab, setTab] = useState(SCREENS[0]!.id);
  const current = SCREENS.find((s) => s.id === tab) ?? SCREENS[0]!;
  return (
    <section id="app" className="scroll-mt-20">
      <div className="mx-auto max-w-6xl px-5 py-24">
        <SectionHeader title="E um app para você ver tudo">
          Telas reais, sem número inventado: o que aparece vem do seu projeto.
        </SectionHeader>
        <Tabs value={tab} onValueChange={setTab} className="mt-10">
          <TabsList className="h-auto flex-wrap gap-1 rounded-xl p-1">
            {SCREENS.map((s) => (
              <TabsTrigger key={s.id} value={s.id} className="gap-2 rounded-lg px-3 py-1.5">
                <s.icon className="size-4" aria-hidden="true" />
                {s.label}
              </TabsTrigger>
            ))}
          </TabsList>
        </Tabs>
        <div className="mt-6">
          <Bento
            columns={1}
            rowHeight={150}
            items={[
              {
                // `content` (não `visual`): a área visual do MagicBento não cresce com a imagem.
                content: (
                  <figure className="flex flex-col gap-4">
                    <Shot
                      key={current.id}
                      src={current.src}
                      alt={current.alt}
                      className="aspect-[16/10] h-auto"
                    />
                    <figcaption className="text-muted-foreground">{current.caption}</figcaption>
                  </figure>
                ),
              },
            ]}
          />
        </div>
      </div>
    </section>
  );
}
