import ScrollReveal from "@/components/ScrollReveal";
import { site } from "@/site";

import { Logo } from "./Nav";

/** Uma carta curta de quem fez: o texto se revela conforme a pessoa rola (ScrollReveal). */
export function Letter() {
  const name = site.maker.name;
  return (
    <section className="border-y border-border bg-card/40">
      <div className="mx-auto max-w-3xl px-5 py-24">
        <h2 className="font-titulo text-[clamp(2rem,3.8vw,3rem)] leading-[1.08]">
          Por que eu fiz a Nexus
        </h2>
        <ScrollReveal
          baseOpacity={0.15}
          enableBlur
          baseRotation={2}
          blurStrength={6}
          containerClassName="mt-8"
          textClassName="text-[clamp(1.25rem,2.2vw,1.6rem)] leading-[1.55] font-medium"
        >
          Eu via o agente de IA fazer coisas incríveis e, minutos depois, errar feio por não saber
          de uma decisão que o time tomou meses antes. Não queria mandar meu código para mais um
          serviço só para resolver isso. Então fiz o contrário: uma ferramenta que mora no seu
          computador, lê o projeto com calma e sempre mostra de onde tirou cada coisa.
        </ScrollReveal>
        <div className="mt-10 flex items-center gap-3">
          <Logo className="size-11" />
          <p className="font-nota text-xl">
            {name ? (
              site.maker.url ? (
                <a href={site.maker.url} className="underline-offset-4 hover:underline">
                  {name}
                </a>
              ) : (
                name
              )
            ) : (
              "Quem fez a Nexus"
            )}
          </p>
        </div>
      </div>
    </section>
  );
}
