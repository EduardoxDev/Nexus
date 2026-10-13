import AnimatedContent from "@/components/AnimatedContent";
import CardSwap, { type CardSwapItem } from "@/components/CardSwap";
import { useTheme } from "@/lib/useTheme";

const SCREENS: CardSwapItem[] = [
  { title: "Visão geral", image: "/screens/visao-geral-escuro.webp", alt: "Visão geral do app" },
  { title: "Busca", image: "/screens/busca-com-previa.webp", alt: "Busca com motivos" },
  { title: "Context Pack", image: "/screens/context-pack.webp", alt: "Context Pack de uma tarefa" },
  { title: "Memória", image: "/screens/memoria.webp", alt: "Memória do projeto" },
  { title: "Integrações", image: "/screens/integracoes.webp", alt: "Integração com o Claude Code" },
];

/** O app de verdade: as telas reais se alternam numa pilha de cartões (CardSwap). */
export function AppShot() {
  const theme = useTheme();
  return (
    <section className="relative overflow-hidden">
      <div className="mx-auto grid max-w-6xl items-center gap-12 px-5 py-24 lg:grid-cols-[0.85fr_1.15fr]">
        <AnimatedContent distance={40} duration={0.8}>
          <h2 className="font-titulo text-[clamp(2rem,3.8vw,3rem)] leading-[1.08]">
            E um app para você ver tudo
          </h2>
          <p className="mt-4 max-w-md text-lg leading-relaxed text-muted-foreground">
            Projetos, buscas, Context Packs, memória e alertas num lugar só. Nada de número
            inventado: o que aparece na tela vem do seu projeto.
          </p>
          <p className="mt-4 text-sm text-muted-foreground">
            Passe o mouse para pausar ou arraste para trocar de tela.
          </p>
        </AnimatedContent>
        <div className="relative h-[380px] sm:h-[460px]">
          <CardSwap
            items={SCREENS}
            width={620}
            height={400}
            cardDistance={46}
            verticalDistance={52}
            delay={4200}
            pauseOnHover
            draggable
            theme={theme}
            cardRadius={18}
          />
        </div>
      </div>
    </section>
  );
}
