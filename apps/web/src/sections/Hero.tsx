import { Download } from "lucide-react";

import AnimatedContent from "@/components/AnimatedContent";
import { Bento, Shot } from "@/components/Bento";
import { GithubIcon } from "@/components/icons";
import Magnet from "@/components/Magnet";
import SplitText from "@/components/SplitText";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useTheme } from "@/lib/useTheme";
import { downloadHref, site } from "@/site";

export function Hero() {
  const theme = useTheme();
  return (
    <section className="relative isolate overflow-hidden">
      <div className="mx-auto max-w-6xl px-5 pt-20 text-center sm:pt-28">
        <AnimatedContent distance={20} duration={0.7}>
          <Badge variant="secondary" className="rounded-full px-3 py-1 text-[13px]">
            Servidor MCP para o Claude Code
          </Badge>
        </AnimatedContent>

        <SplitText
          tag="h1"
          text="O contexto do seu código, na mão do seu agente de IA."
          className="mx-auto mt-7 max-w-4xl font-titulo text-[clamp(2.5rem,5.6vw,4.4rem)] leading-[1.03]"
          textAlign="center"
          splitType="words"
          delay={40}
          duration={0.7}
          from={{ opacity: 0, y: 24 }}
          to={{ opacity: 1, y: 0 }}
        />

        <AnimatedContent distance={20} duration={0.8} delay={0.3}>
          <p className="mx-auto mt-6 max-w-2xl text-lg leading-relaxed text-muted-foreground sm:text-xl">
            A Nexus lê seus repositórios no seu computador e entrega ao Claude Code o trecho certo,
            a linha exata e as decisões do time. Seu código não sai da sua máquina.
          </p>
          <div className="mt-9 flex flex-wrap items-center justify-center gap-3">
            <Magnet padding={60} magnetStrength={4}>
              <Button size="lg" className="h-12 rounded-xl px-6 text-base" asChild>
                <a href={downloadHref || "#comecar"}>
                  <Download className="size-5" />
                  Baixar para Windows
                </a>
              </Button>
            </Magnet>
            {site.githubUrl ? (
              <Button
                size="lg"
                variant="outline"
                className="h-12 rounded-xl px-6 text-base"
                asChild
              >
                <a href={site.githubUrl} rel="noopener">
                  <GithubIcon className="size-5" />
                  Ver o código
                </a>
              </Button>
            ) : (
              <Button
                size="lg"
                variant="outline"
                className="h-12 rounded-xl px-6 text-base"
                asChild
              >
                <a href="#o-que-faz">Ver o que ela faz</a>
              </Button>
            )}
          </div>
          <p className="mt-5 text-[15px] text-muted-foreground">Grátis e de código aberto.</p>
        </AnimatedContent>
      </div>

      <div className="mx-auto mt-16 max-w-6xl px-5">
        <AnimatedContent distance={60} duration={1} delay={0.4}>
          <Bento
            columns={1}
            rowHeight={150}
            items={[
              {
                // `content` (não `visual`): a área visual do MagicBento não cresce com a imagem.
                content: (
                  <Shot
                    src={
                      theme === "light"
                        ? "/screens/visao-geral-claro.webp"
                        : "/screens/visao-geral-escuro.webp"
                    }
                    alt="Visão geral do app Nexus: indicadores, projetos e operações recentes"
                    className="aspect-[16/10] h-auto"
                  />
                ),
              },
            ]}
          />
        </AnimatedContent>
      </div>
    </section>
  );
}
