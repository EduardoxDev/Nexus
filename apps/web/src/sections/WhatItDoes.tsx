import { BookOpenCheck, Cable, Code2, PackageOpen, Radar, ScanSearch } from "lucide-react";

import AnimatedContent from "@/components/AnimatedContent";
import MagicBento, { type MagicBentoItem } from "@/components/MagicBento";
import { useTheme } from "@/lib/useTheme";

function Shot({ src, alt }: { src: string; alt: string }) {
  return (
    <img
      src={src}
      alt={alt}
      loading="lazy"
      width={1440}
      height={900}
      className="h-44 w-full rounded-lg border border-white/10 object-cover object-top sm:h-52"
    />
  );
}

const ITEMS: MagicBentoItem[] = [
  {
    title: "Encontra o trecho certo",
    description:
      "Pergunte em português ou inglês. A resposta traz o arquivo, as linhas e o motivo — e avisa se o arquivo mudou.",
    icon: <ScanSearch className="size-5" />,
    visual: (
      <Shot src="/screens/busca-com-previa.webp" alt="Busca com motivos e prévia verificada" />
    ),
    colSpan: 2,
    rowSpan: 2,
  },
  {
    title: "Feita para agentes",
    description: "Servidor MCP local com 13 ferramentas. O Claude Code consulta a Nexus sozinho.",
    icon: <Cable className="size-5" />,
  },
  {
    title: "Lembra das decisões",
    description: "Você confirma o que vale e isso chega ao agente junto com o código.",
    icon: <BookOpenCheck className="size-5" />,
  },
  {
    title: "Monta o contexto de uma tarefa",
    description:
      "Arquivos certos, testes e decisões num pacote só, do tamanho que o agente aguenta.",
    icon: <PackageOpen className="size-5" />,
    visual: <Shot src="/screens/context-pack.webp" alt="Context Pack compilado para uma tarefa" />,
    colSpan: 2,
    rowSpan: 2,
  },
  {
    title: "Avisa quando algo envelheceu",
    description:
      "Documento citando arquivo que sumiu, decisão que mudou, regras que se contradizem.",
    icon: <Radar className="size-5" />,
  },
  {
    title: "Entende o código",
    description:
      "TypeScript, JavaScript, Python e Rust: quem importa quem e onde cada função nasce.",
    icon: <Code2 className="size-5" />,
  },
];

export function WhatItDoes() {
  const theme = useTheme();
  return (
    <section id="o-que-faz" className="scroll-mt-20">
      <div className="mx-auto max-w-6xl px-5 py-24">
        <AnimatedContent distance={40} duration={0.8}>
          <h2 className="max-w-2xl font-titulo text-[clamp(2rem,3.8vw,3rem)] leading-[1.08]">
            O que ela faz por você
          </h2>
          <p className="mt-4 max-w-xl text-lg text-muted-foreground">
            Pense numa bibliotecária que leu o seu projeto inteiro e sempre diz de onde tirou cada
            coisa.
          </p>
        </AnimatedContent>
        <div className="mt-12">
          <MagicBento
            items={ITEMS}
            columns={4}
            rowHeight={200}
            gap={14}
            radius={20}
            theme={theme}
            glowColor="79, 111, 255"
            enableStars
            enableSpotlight
            enableBorderGlow
            enableTilt={false}
            enableMagnetism={false}
            clickEffect
            particleCount={10}
            spotlightRadius={320}
            textAutoHide={false}
          />
        </div>
      </div>
    </section>
  );
}
