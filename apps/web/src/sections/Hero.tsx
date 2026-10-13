import { ArrowDown } from "lucide-react";
import { motion } from "motion/react";
import { Fragment, useEffect, useState } from "react";

import AnimatedContent from "@/components/AnimatedContent";
import { Bento, Shot } from "@/components/Bento";
import ClickSpark from "@/components/ClickSpark";
import FaultyTerminal from "@/components/FaultyTerminal";
import { DownloadButton, GhostButton } from "@/components/Cta";
import { GithubIcon } from "@/components/icons";
import Magnet from "@/components/Magnet";
import { Badge } from "@/components/ui/badge";
import { HoverCard, HoverCardContent, HoverCardTrigger } from "@/components/ui/hover-card";
import { useTheme } from "@/lib/useTheme";
import { site } from "@/site";
import { asset } from "@/lib/asset";

const LINES = ["O contexto do seu código,", "na mão do seu agente de IA."];

/** Fundo WebGL só no navegador (o HTML pré-renderizado sai sem ele). */
function useMounted() {
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);
  return mounted;
}

export function Hero() {
  const theme = useTheme();
  const mounted = useMounted();
  return (
    <section className="relative isolate overflow-hidden">
      <div aria-hidden="true" className="absolute inset-x-0 top-0 -z-10 h-[760px]">
        {mounted && (
          <FaultyTerminal
            tint={theme === "light" ? "#2348ce" : "#4f6fff"}
            lightMode={theme === "light"}
            glyphs="blocks"
            scale={1.5}
            digitSize={1.2}
            timeScale={0.5}
            brightness={theme === "light" ? 0.35 : 0.55}
            glow={0.5}
            persistence={0.5}
            mouseStrength={0.5}
            mouseGlow={0.6}
            curvature={0.1}
            vignette={0.25}
            scanlineIntensity={0.3}
            clickEffect="glitch"
          />
        )}
        {/* Clareia o meio (onde está o texto) e funde com o fundo sólido embaixo. */}
        <div className="absolute inset-0 bg-[radial-gradient(ellipse_60%_45%_at_50%_42%,var(--background)_15%,transparent_75%)] opacity-80" />
        <div className="absolute inset-x-0 bottom-0 h-72 bg-gradient-to-b from-transparent to-background" />
      </div>
      <div className="mx-auto max-w-6xl px-5 pt-20 text-center sm:pt-28">
        <AnimatedContent distance={20} duration={0.7}>
          <Badge variant="secondary" className="rounded-full px-3 py-1 text-[13px]">
            Servidor MCP para o Claude Code
          </Badge>
        </AnimatedContent>

        <h1 className="mx-auto mt-7 max-w-5xl font-titulo text-[clamp(2.5rem,5.4vw,4.1rem)] leading-[1.05] tracking-[-0.035em]">
          {LINES.map((line, i) => (
            <Fragment key={line}>
              {i > 0 && " "}
              <motion.span
                className={`block ${i === 1 ? "text-muted-foreground" : ""}`}
                initial={{ opacity: 0, y: 18, filter: "blur(10px)" }}
                animate={{ opacity: 1, y: 0, filter: "blur(0px)" }}
                transition={{ duration: 0.9, delay: 0.1 + i * 0.18, ease: [0.22, 1, 0.36, 1] }}
              >
                {line}
              </motion.span>
            </Fragment>
          ))}
        </h1>

        <AnimatedContent distance={20} duration={0.8} delay={0.3}>
          <p className="mx-auto mt-6 max-w-2xl text-lg leading-relaxed text-muted-foreground sm:text-xl">
            A Nexus lê seus repositórios no seu computador e entrega ao{" "}
            <HoverCard openDelay={150}>
              <HoverCardTrigger asChild>
                <a
                  href="https://claude.com/claude-code"
                  rel="noopener"
                  className="text-foreground underline decoration-primary/60 decoration-dotted underline-offset-4 hover:decoration-primary"
                >
                  Claude Code
                </a>
              </HoverCardTrigger>
              <HoverCardContent className="w-80 text-left">
                <p className="font-medium text-foreground">Claude Code</p>
                <p className="mt-1 text-sm leading-relaxed text-muted-foreground">
                  O agente de IA da Anthropic que trabalha no seu terminal. Ele conversa com a Nexus
                  pelo MCP, o protocolo aberto para dar ferramentas a agentes.
                </p>
              </HoverCardContent>
            </HoverCard>{" "}
            o trecho certo, a linha exata e as decisões do time. Seu código não sai da sua máquina.
          </p>
          <ClickSpark
            sparkColor={["#4f6fff", "#9fb4ff", "#ffffff"]}
            sparkCount={10}
            sparkRadius={26}
          >
            <div className="mt-9 flex flex-wrap items-center justify-center gap-3">
              <Magnet padding={60} magnetStrength={5}>
                <DownloadButton />
              </Magnet>
              {site.githubUrl ? (
                <GhostButton href={site.githubUrl} rel="noopener">
                  <GithubIcon className="size-[18px]" />
                  Ver o código
                </GhostButton>
              ) : (
                <GhostButton href="#o-que-faz">
                  Ver o que ela faz
                  <ArrowDown className="size-4" />
                </GhostButton>
              )}
            </div>
          </ClickSpark>
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
                        ? asset("/screens/visao-geral-claro.webp")
                        : asset("/screens/visao-geral-escuro.webp")
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
