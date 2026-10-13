import { EyeOff, FileLock2, Gauge, Link2Off, Timer, WifiOff, Wrench, Zap } from "lucide-react";

import { Bento, type BentoItem } from "@/components/Bento";
import CountUp from "@/components/CountUp";
import { SectionHeader } from "@/components/SectionHeader";

const PRIVACY: BentoItem[] = [
  {
    title: "Sem servidor",
    description: "O índice e a memória ficam num arquivo no seu computador.",
    icon: <WifiOff className="size-5" />,
  },
  {
    title: "Sem telemetria",
    description: "Nenhum dado de uso é coletado. Os registros locais nunca guardam conteúdo.",
    icon: <EyeOff className="size-5" />,
  },
  {
    title: "Segredos ficam de fora",
    description: "Senhas, chaves e .env são reconhecidos e ignorados, mesmo se aparecerem depois.",
    icon: <FileLock2 className="size-5" />,
  },
  {
    title: "Só o que você autorizou",
    description: "Atalhos que apontam para fora do projeto nunca são seguidos.",
    icon: <Link2Off className="size-5" />,
  },
];

function Number({
  to,
  prefix = "",
  suffix = "",
  decimals = 0,
}: {
  to: number;
  prefix?: string;
  suffix?: string;
  decimals?: number;
}) {
  return (
    <p className="font-titulo text-5xl text-primary">
      {prefix}
      {decimals > 0 ? (
        to.toLocaleString("pt-BR", { minimumFractionDigits: decimals })
      ) : (
        <CountUp to={to} duration={1.6} />
      )}
      {suffix}
    </p>
  );
}

/** Medições reais (docs/benchmarks.md, docs/evaluation.md). */
const NUMBERS: BentoItem[] = [
  {
    title: "para indexar 10 mil arquivos",
    visual: <Number to={3} prefix="~" suffix=" s" />,
    icon: <Zap className="size-5" />,
  },
  {
    title: "para atualizar depois de salvar",
    visual: <Number to={0.4} suffix=" s" decimals={1} />,
    icon: <Timer className="size-5" />,
  },
  {
    title: "por busca, na mediana",
    visual: <Number to={37} suffix=" ms" />,
    icon: <Gauge className="size-5" />,
  },
  {
    title: "ferramentas para agentes",
    visual: <Number to={13} />,
    icon: <Wrench className="size-5" />,
  },
];

export function Privacy() {
  return (
    <section id="privacidade" className="scroll-mt-20">
      <div className="mx-auto max-w-6xl px-5 py-24">
        <SectionHeader title="Seu código fica no seu computador">
          Quando você usa um agente na nuvem, ele envia ao provedor o que ler — como já faz hoje. A
          Nexus só entrega o que você autorizou.
        </SectionHeader>
        <div className="mt-12">
          <Bento items={PRIVACY} columns={4} rowHeight={190} />
        </div>
        <div className="mt-14">
          <SectionHeader title="Rápida de verdade">
            Medido num PC comum com Windows. Os scripts estão no repositório.
          </SectionHeader>
          <div className="mt-12">
            <Bento items={NUMBERS} columns={4} rowHeight={190} />
          </div>
        </div>
      </div>
    </section>
  );
}
