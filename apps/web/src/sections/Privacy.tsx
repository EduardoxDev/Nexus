import { EyeOff, FileLock2, Link2Off, WifiOff } from "lucide-react";
import type { ReactNode } from "react";

import AnimatedContent from "@/components/AnimatedContent";
import { Bento } from "@/components/Bento";
import CountUp from "@/components/CountUp";
import { SectionHeader } from "@/components/SectionHeader";

const PRIVACY = [
  {
    title: "Sem servidor",
    text: "O índice e a memória ficam num arquivo no seu computador.",
    icon: WifiOff,
  },
  {
    title: "Sem telemetria",
    text: "Nenhum dado de uso é coletado. Os registros locais nunca guardam conteúdo.",
    icon: EyeOff,
  },
  {
    title: "Segredos ficam de fora",
    text: "Senhas, chaves e .env são reconhecidos e ignorados, mesmo se aparecerem depois.",
    icon: FileLock2,
  },
  {
    title: "Só o que você autorizou",
    text: "Atalhos que apontam para fora do projeto nunca são seguidos.",
    icon: Link2Off,
  },
];

/** As quatro garantias numa lista só, separadas por linhas finas. */
function Guarantees() {
  return (
    <ul className="divide-y divide-border">
      {PRIVACY.map(({ title, text, icon: Icon }) => (
        <li key={title} className="flex gap-4 py-5 first:pt-1 last:pb-1">
          <span className="grid size-10 flex-none place-items-center rounded-xl border border-primary/25 bg-primary/10 text-primary">
            <Icon className="size-[18px]" />
          </span>
          <div>
            <h3 className="font-semibold tracking-tight text-foreground">{title}</h3>
            <p className="mt-1 text-[15px] leading-relaxed text-muted-foreground">{text}</p>
          </div>
        </li>
      ))}
    </ul>
  );
}

function Stat({ value, unit, label }: { value: ReactNode; unit?: string; label: string }) {
  return (
    <div className="px-6 py-7 first:pl-2 sm:py-2">
      <p className="font-titulo text-[clamp(2.4rem,4vw,3.25rem)] leading-none text-foreground tabular-nums">
        {value}
        {unit && <span className="ml-1 text-[0.55em] text-primary">{unit}</span>}
      </p>
      <p className="mt-3 text-[15px] leading-snug text-muted-foreground">{label}</p>
    </div>
  );
}

/** Medições reais (docs/benchmarks.md, docs/evaluation.md). */
function Numbers() {
  return (
    <div className="grid divide-y divide-border sm:grid-cols-2 sm:divide-y-0 lg:grid-cols-4 lg:divide-x">
      <Stat
        value={
          <>
            ~<CountUp to={3} duration={1.2} />
          </>
        }
        unit="s"
        label="para indexar 10 mil arquivos"
      />
      <Stat value="0,4" unit="s" label="para atualizar depois de salvar" />
      <Stat value={<CountUp to={37} duration={1.6} />} unit="ms" label="por busca, na mediana" />
      <Stat value={<CountUp to={13} duration={1.4} />} label="ferramentas para agentes" />
    </div>
  );
}

export function Privacy() {
  return (
    <section id="privacidade" className="scroll-mt-24">
      <div className="mx-auto max-w-6xl px-5 py-24">
        <div className="grid items-start gap-12 lg:grid-cols-[1fr_1.15fr] lg:gap-16">
          <div className="lg:sticky lg:top-28">
            <SectionHeader
              title="Seu código fica no seu computador."
              accent="Nada sai sem você autorizar."
            >
              Quando você usa um agente na nuvem, ele envia ao provedor o que ler — como já faz
              hoje. A Nexus só entrega o que você autorizou.
            </SectionHeader>
          </div>
          <AnimatedContent distance={30} duration={0.8}>
            <Bento columns={1} rowHeight={120} items={[{ content: <Guarantees /> }]} />
          </AnimatedContent>
        </div>

        <div className="mt-24">
          <SectionHeader title="Rápida de verdade." accent="Medida, não prometida.">
            Medido num PC comum com Windows. Os scripts estão no repositório.
          </SectionHeader>
          <AnimatedContent distance={30} duration={0.8} className="mt-10">
            <Bento columns={1} rowHeight={120} items={[{ content: <Numbers /> }]} />
          </AnimatedContent>
        </div>
      </div>
    </section>
  );
}
