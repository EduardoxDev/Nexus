import { BookOpenCheck, Cable, Code2, PackageOpen, Radar, ScanSearch } from "lucide-react";

import { Bento, Shot, type BentoItem } from "@/components/Bento";
import { SectionHeader } from "@/components/SectionHeader";

/** A seção aprovada: o mesmo conteúdo e o mesmo MagicBento (agora via <Bento>). */
const ITEMS: BentoItem[] = [
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
  return (
    <section id="o-que-faz" className="scroll-mt-20">
      <div className="mx-auto max-w-6xl px-5 py-24">
        <SectionHeader title="O que ela faz por você">
          Pense numa bibliotecária que leu o seu projeto inteiro e sempre diz de onde tirou cada
          coisa.
        </SectionHeader>
        <div className="mt-12">
          <Bento items={ITEMS} stars />
        </div>
      </div>
    </section>
  );
}
