import { Check, X } from "lucide-react";

import { Reveal } from "@/components/fx";

const BEFORE = [
  "O agente abre arquivo por arquivo e ainda assim perde a peça que importa.",
  "Decisões do time ficam em ADRs e conversas que ninguém lembra de mencionar.",
  "Documentação desatualizada é lida como verdade — e vira código errado.",
];

const AFTER = [
  "Busca híbrida explica por que cada trecho apareceu e confere se ele ainda é atual.",
  "Memória do projeto com decisões que você confirmou, entregue junto com o código.",
  "Alertas de atualidade apontam o que mudou, sumiu ou se contradiz.",
];

export function Problem() {
  return (
    <section className="relative py-16 sm:py-20">
      <div className="mx-auto max-w-6xl px-5">
        <Reveal className="mx-auto max-w-3xl text-center">
          <p className="text-sm font-semibold tracking-[0.16em] text-primary uppercase">
            O problema
          </p>
          <h2 className="mt-3 font-display text-4xl font-bold sm:text-5xl">
            Seu agente lê arquivos.
            <br />
            <span className="text-muted-foreground">Ele não conhece o seu projeto.</span>
          </h2>
        </Reveal>

        <div className="mt-14 grid gap-5 md:grid-cols-2">
          <Reveal className="rounded-2xl border border-border bg-card/60 p-7">
            <p className="text-sm font-semibold text-muted-foreground">Sem a NEXUS</p>
            <ul className="mt-5 space-y-4">
              {BEFORE.map((t) => (
                <li
                  key={t}
                  className="flex gap-3 text-[15px] leading-relaxed text-muted-foreground"
                >
                  <span className="mt-0.5 flex size-5 shrink-0 items-center justify-center rounded-full bg-muted">
                    <X className="size-3" aria-hidden="true" />
                  </span>
                  {t}
                </li>
              ))}
            </ul>
          </Reveal>
          <Reveal
            delay={0.1}
            className="relative overflow-hidden rounded-2xl border border-primary/25 bg-card p-7 shadow-[0_20px_60px_-30px_oklch(0.5_0.2_266/0.5)]"
          >
            <div
              aria-hidden="true"
              className="absolute -top-24 -right-24 size-64 rounded-full bg-[radial-gradient(closest-side,var(--glow),transparent)]"
            />
            <p className="relative text-sm font-semibold text-primary">Com a NEXUS</p>
            <ul className="relative mt-5 space-y-4">
              {AFTER.map((t) => (
                <li key={t} className="flex gap-3 text-[15px] leading-relaxed">
                  <span className="mt-0.5 flex size-5 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground">
                    <Check className="size-3" aria-hidden="true" />
                  </span>
                  {t}
                </li>
              ))}
            </ul>
          </Reveal>
        </div>
      </div>
    </section>
  );
}
