import { useEffect, useState } from "react";

import Aurora from "@/components/Aurora";
import { EvidenceCard } from "@/components/EvidenceCard";
import { GithubIcon } from "@/components/icons";
import Magnet from "@/components/Magnet";
import { Mascot } from "@/components/Mascot";
import RotatingText from "@/components/RotatingText";
import ShinyText from "@/components/ShinyText";
import SplitText from "@/components/SplitText";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useTheme } from "@/lib/useTheme";
import { downloadHref, site } from "@/site";

/** Fundo WebGL só no navegador (no HTML pré-renderizado fica o degradê de reserva). */
function useMounted() {
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);
  return mounted;
}

export function Hero() {
  const mounted = useMounted();
  const theme = useTheme();
  return (
    <section className="relative isolate overflow-hidden">
      <div
        aria-hidden="true"
        className="absolute inset-x-0 top-0 -z-10 h-[620px] bg-[radial-gradient(60%_80%_at_50%_0%,rgba(79,111,255,0.35),transparent_70%)]"
      >
        {mounted && (
          <Aurora
            colorStops={["#0b1a5c", "#2348ce", "#14287a"]}
            amplitude={0.9}
            blend={0.45}
            speed={0.5}
            brightness={0.55}
            glow={0.25}
            opacity={0.85}
            lightMode={theme === "light"}
          />
        )}
      </div>
      <div
        aria-hidden="true"
        className="absolute inset-x-0 top-[420px] -z-10 h-[260px] bg-gradient-to-b from-transparent to-background"
      />

      <div className="mx-auto grid max-w-6xl items-center gap-x-14 gap-y-16 px-5 pt-16 pb-24 sm:pt-24 lg:grid-cols-[1.15fr_1fr]">
        <div>
          <Badge variant="secondary" className="rounded-full px-3 py-1 text-[13px]">
            <ShinyText text="Novo: servidor MCP para o Claude Code" speed={3} />
          </Badge>

          <SplitText
            tag="h1"
            text="Seu agente de IA não conhece o seu projeto. A Nexus conhece."
            className="mt-6 font-titulo text-[clamp(2.5rem,5vw,4rem)] leading-[1.04]"
            textAlign="left"
            splitType="words"
            delay={45}
            duration={0.7}
            from={{ opacity: 0, y: 28, filter: "blur(8px)" }}
            to={{ opacity: 1, y: 0, filter: "blur(0px)" }}
          />

          <p className="mt-6 max-w-[34rem] text-lg leading-relaxed text-muted-foreground sm:text-xl">
            Ela lê seus repositórios aqui no seu computador e entrega ao Claude Code
          </p>
          <div className="mt-2 flex flex-wrap items-center gap-2 text-lg sm:text-xl">
            <RotatingText
              texts={[
                "o trecho certo",
                "a linha exata",
                "o porquê de cada resposta",
                "as decisões do time",
              ]}
              mainClassName="overflow-hidden rounded-lg bg-primary px-2.5 py-0.5 font-titulo text-primary-foreground"
              staggerFrom="last"
              initial={{ y: "100%" }}
              animate={{ y: 0 }}
              exit={{ y: "-120%" }}
              staggerDuration={0.02}
              splitLevelClassName="overflow-hidden pb-0.5"
              transition={{ type: "spring", damping: 30, stiffness: 400 }}
              rotationInterval={2400}
            />
            <span className="text-muted-foreground">— sem mandar seu código pra lugar nenhum.</span>
          </div>

          <div className="mt-10 flex flex-wrap items-center gap-3">
            <Magnet padding={60} magnetStrength={4}>
              <Button size="lg" className="h-12 rounded-xl px-6 text-base" asChild>
                <a href={downloadHref || "#comecar"}>Baixar para Windows</a>
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
        </div>

        {/* A nuvem espia a resposta por cima do cartão, com os olhos à vista. */}
        <div className="relative pt-24 sm:pt-28">
          <EvidenceCard />
          <Mascot className="absolute top-0 right-3 z-10 w-36 rotate-[7deg] drop-shadow-[0_16px_28px_rgba(35,72,206,0.45)] sm:-right-4 sm:w-44" />
        </div>
      </div>
    </section>
  );
}
