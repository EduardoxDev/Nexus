import { EyeOff, FileLock2, Link2Off, WifiOff, type LucideIcon } from "lucide-react";

import AnimatedContent from "@/components/AnimatedContent";
import SpotlightCard from "@/components/SpotlightCard";
import { useTheme } from "@/lib/useTheme";

const POINTS: { icon: LucideIcon; title: string; body: string }[] = [
  {
    icon: WifiOff,
    title: "Sem servidor",
    body: "Não existe servidor da Nexus. O índice e a memória ficam num arquivo no seu computador.",
  },
  {
    icon: EyeOff,
    title: "Sem telemetria",
    body: "Nenhum dado de uso é coletado. Os registros locais nunca guardam o conteúdo dos arquivos.",
  },
  {
    icon: FileLock2,
    title: "Segredos ficam de fora",
    body: "Senhas, chaves e .env são reconhecidos e ignorados, mesmo se aparecerem depois.",
  },
  {
    icon: Link2Off,
    title: "Só o que você autorizou",
    body: "Atalhos que apontam para fora do projeto nunca são seguidos.",
  },
];

export function Privacy() {
  const theme = useTheme();
  return (
    <section id="privacidade" className="scroll-mt-20">
      <div className="mx-auto grid max-w-6xl gap-12 px-5 py-24 lg:grid-cols-[0.9fr_1.1fr]">
        <AnimatedContent distance={40} duration={0.8}>
          <h2 className="font-titulo text-[clamp(2.2rem,4.4vw,3.4rem)] leading-[1.04]">
            Seu código fica no seu computador.
          </h2>
          <p className="mt-6 max-w-lg text-lg leading-relaxed text-muted-foreground">
            Quando você usa um agente na nuvem, ele envia ao provedor o que ler — como já faz hoje.
            A Nexus só entrega o que você autorizou, e a tela Integrações mostra cada consulta: qual
            ferramenta, quando e o resultado.
          </p>
        </AnimatedContent>
        <div className="grid gap-4 sm:grid-cols-2">
          {POINTS.map((p, i) => (
            <AnimatedContent key={p.title} distance={40} delay={i * 0.08} duration={0.7}>
              <SpotlightCard
                theme={theme}
                spotlightColor="rgba(79, 111, 255, 0.28)"
                className="h-full rounded-2xl p-6"
              >
                <p.icon className="size-6 text-primary" aria-hidden="true" />
                <h3 className="mt-4 font-titulo text-lg">{p.title}</h3>
                <p className="mt-1.5 leading-relaxed text-muted-foreground">{p.body}</p>
              </SpotlightCard>
            </AnimatedContent>
          ))}
        </div>
      </div>
    </section>
  );
}
