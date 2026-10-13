import { Heart } from "lucide-react";

import { Reveal, SpotlightCard } from "@/components/fx";
import { site } from "@/site";

import { Logo } from "./Nav";

/** Detalhes de cuidado que existem de verdade no projeto. */
const CARE = [
  "Feita em português, do primeiro botão à documentação.",
  "Mais de 350 testes automáticos — inclusive o app real sendo aberto e usado.",
  "Contraste acessível nos dois temas e respeito ao “movimento reduzido”.",
  "Nenhum número inventado: o que aparece na tela vem do seu projeto.",
];

export function Story() {
  const maker = site.maker.name;
  return (
    <section className="relative py-16 sm:py-20">
      <div className="mx-auto max-w-4xl px-5">
        <Reveal>
          <SpotlightCard className="p-8 sm:p-12" color="oklch(0.65 0.2 15 / 0.10)">
            <div className="flex items-center gap-3">
              <Logo className="size-10" />
              <Heart
                className="size-5 fill-[oklch(0.65_0.2_15)] text-[oklch(0.65_0.2_15)]"
                aria-hidden="true"
              />
            </div>
            <h2 className="mt-6 font-display text-3xl font-bold sm:text-4xl">
              Feito com o coração.
            </h2>
            <div className="mt-5 space-y-4 text-[17px] leading-relaxed text-muted-foreground">
              <p>
                A NEXUS nasceu de uma frustração simples: ver um agente de IA brilhante errar porque
                não sabia de uma decisão tomada meses antes, ou porque confiou num documento que
                ninguém atualizou.
              </p>
              <p>
                Em vez de mandar o seu código para mais um serviço, a ideia foi o contrário: dar ao
                agente um bom bibliotecário <em>dentro</em> do seu computador — que mostra de onde
                tirou cada coisa e avisa quando algo envelheceu.
              </p>
            </div>
            <ul className="mt-8 grid gap-3 sm:grid-cols-2">
              {CARE.map((c) => (
                <li key={c} className="flex gap-2.5 text-sm">
                  <Heart className="mt-0.5 size-4 shrink-0 text-primary" aria-hidden="true" />
                  <span>{c}</span>
                </li>
              ))}
            </ul>
            <p className="mt-8 text-sm text-muted-foreground">
              {maker ? (
                <>
                  —{" "}
                  {site.maker.url ? (
                    <a
                      href={site.maker.url}
                      className="font-medium text-foreground underline-offset-4 hover:underline"
                    >
                      {maker}
                    </a>
                  ) : (
                    <span className="font-medium text-foreground">{maker}</span>
                  )}
                  , criador da NEXUS
                </>
              ) : (
                <>— Feita no Brasil, com café e cuidado.</>
              )}
            </p>
          </SpotlightCard>
        </Reveal>
      </div>
    </section>
  );
}
