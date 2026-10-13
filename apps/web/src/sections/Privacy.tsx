import { EyeOff, FileLock2, Link2Off, ShieldCheck, WifiOff, type LucideIcon } from "lucide-react";

import { Reveal } from "@/components/fx";

const ITEMS: { icon: LucideIcon; title: string; body: string }[] = [
  {
    icon: WifiOff,
    title: "Sem rede, sem nuvem",
    body: "O índice, a memória e as buscas ficam num banco local. Não existe servidor da NEXUS.",
  },
  {
    icon: EyeOff,
    title: "Sem telemetria",
    body: "Nenhum dado de uso é coletado. Os logs locais nunca guardam o conteúdo dos arquivos.",
  },
  {
    icon: FileLock2,
    title: "Segredos ficam de fora",
    body: "Chaves, tokens e .env são detectados e nunca têm o conteúdo indexado — nem se aparecerem depois.",
  },
  {
    icon: Link2Off,
    title: "Só o que você autorizou",
    body: "Links e junctions para fora do projeto nunca são seguidos, nem na indexação nem nas releituras.",
  },
];

export function Privacy() {
  return (
    <section id="privacidade" className="relative scroll-mt-20 py-16 sm:py-20">
      <div className="mx-auto grid max-w-6xl items-center gap-14 px-5 lg:grid-cols-[1fr_1.1fr]">
        <Reveal>
          <span className="flex size-14 items-center justify-center rounded-2xl border border-success/30 bg-success/10 text-success">
            <ShieldCheck className="size-7" aria-hidden="true" />
          </span>
          <h2 className="mt-6 font-display text-4xl font-bold sm:text-5xl">
            Seu código não sai da sua máquina.
          </h2>
          <p className="mt-5 text-lg leading-relaxed text-muted-foreground">
            Privacidade não é uma opção nas configurações — é a arquitetura. A NEXUS roda inteira no
            seu computador e entrega aos agentes só o que você autorizou, marcado como dado, nunca
            como instrução.
          </p>
          <p className="mt-4 text-sm text-muted-foreground">
            Quando você usa um agente na nuvem, ele envia ao provedor o que ler — como já faz hoje.
            A tela Integrações mostra cada consulta do agente: qual ferramenta, quando e o
            resultado.
          </p>
        </Reveal>
        <div className="grid gap-4 sm:grid-cols-2">
          {ITEMS.map((it, i) => (
            <Reveal
              key={it.title}
              delay={i * 0.08}
              className="rounded-2xl border border-border bg-card p-6 transition-colors hover:border-success/30"
            >
              <it.icon className="size-5 text-success" aria-hidden="true" />
              <h3 className="mt-4 font-semibold">{it.title}</h3>
              <p className="mt-1.5 text-sm leading-relaxed text-muted-foreground">{it.body}</p>
            </Reveal>
          ))}
        </div>
      </div>
    </section>
  );
}
