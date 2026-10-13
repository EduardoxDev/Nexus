import { Check, Copy, Download, FolderGit2, Terminal } from "lucide-react";
import { useState } from "react";

import { Bento, type BentoItem } from "@/components/Bento";
import { SectionHeader } from "@/components/SectionHeader";
import { Button } from "@/components/ui/button";

const COMMAND = "claude mcp add nexus -- nexus mcp serve";

function CopyCommand() {
  const [copied, setCopied] = useState(false);
  const copy = () => {
    void navigator.clipboard
      .writeText(COMMAND)
      .then(() => {
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1600);
      })
      .catch(() => setCopied(false));
  };
  return (
    <div className="flex w-full items-center gap-3 rounded-xl border border-white/10 bg-black/40 py-2 pr-2 pl-4 text-left text-[#dbe4ff]">
      <span aria-hidden="true" className="font-codigo text-[#7ee2a8]">
        $
      </span>
      <code className="font-codigo min-w-0 flex-1 overflow-x-auto text-[15px] whitespace-nowrap">
        {COMMAND}
      </code>
      <Button size="sm" variant="secondary" onClick={copy} aria-label="Copiar comando">
        {copied ? <Check /> : <Copy />}
        {copied ? "Copiado" : "Copiar"}
      </Button>
    </div>
  );
}

/** É de fato uma sequência: os títulos trazem a ordem. */
const ITEMS: BentoItem[] = [
  {
    title: "1. Instale e escolha uma pasta",
    description:
      "O instalador não pede administrador. Você aponta para o projeto; a Nexus nunca lê nada fora dele.",
    icon: <Download className="size-5" />,
  },
  {
    title: "2. Espere alguns segundos",
    description:
      "Ela indexa sozinha e deixa de fora senhas, chaves e binários. Depois, só relê o que mudou.",
    icon: <FolderGit2 className="size-5" />,
  },
  {
    title: "3. Conecte o Claude Code",
    description:
      "Rode o comando uma vez. A tela Integrações do app mostra a versão com os seus caminhos.",
    icon: <Terminal className="size-5" />,
  },
  {
    title: "Um comando e pronto",
    description: "O instalador já traz a linha de comando nexus. Sem conta, sem chave de API.",
    visual: <CopyCommand />,
    colSpan: 3,
  },
];

export function GetStarted() {
  return (
    <section id="comecar" className="scroll-mt-20">
      <div className="mx-auto max-w-6xl px-5 py-24">
        <SectionHeader title="Em três passos, sem criar conta">
          Instale, aponte para o projeto e conecte o seu agente.
        </SectionHeader>
        <div className="mt-12">
          <Bento items={ITEMS} columns={3} rowHeight={190} />
        </div>
      </div>
    </section>
  );
}
