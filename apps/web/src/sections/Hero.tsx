import { ArrowRight, Download, EyeOff, Lock, Monitor } from "lucide-react";
import { motion, useReducedMotion, useScroll, useTransform } from "motion/react";
import { useRef } from "react";

import { Aurora, BlurText, BorderBeam, Magnet, ShinyText } from "@/components/fx";
import { GithubIcon } from "@/components/icons";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { downloadHref, site } from "@/site";

const EASE = [0.22, 1, 0.36, 1] as const;

/** Janela do app com a captura real (escura ou clara conforme o tema). */
function ProductShot() {
  const ref = useRef<HTMLDivElement>(null);
  const reduce = useReducedMotion();
  const { scrollYProgress } = useScroll({ target: ref, offset: ["start end", "start 30%"] });
  const rotateX = useTransform(scrollYProgress, [0, 1], [reduce ? 0 : 18, 0]);
  const scale = useTransform(scrollYProgress, [0, 1], [reduce ? 1 : 0.92, 1]);
  return (
    <div ref={ref} className="mx-auto mt-16 w-full max-w-5xl [perspective:1400px] sm:mt-20">
      <motion.div
        style={{ rotateX, scale, transformOrigin: "50% 100%" }}
        className="relative rounded-2xl border border-border bg-card p-1.5 shadow-[0_40px_120px_-30px_oklch(0.45_0.2_266/0.55)]"
      >
        <BorderBeam />
        <div className="flex items-center gap-1.5 px-3 pt-1.5 pb-2.5">
          <span className="size-2.5 rounded-full bg-[#ff5f57]" />
          <span className="size-2.5 rounded-full bg-[#febc2e]" />
          <span className="size-2.5 rounded-full bg-[#28c840]" />
          <span className="ml-3 truncate text-xs text-muted-foreground">NEXUS — Visão geral</span>
        </div>
        <img
          src="/screens/visao-geral-escuro.webp"
          alt="Visão geral do app NEXUS: indicadores do projeto, operações recentes e estado do sistema"
          width={1440}
          height={900}
          className="hidden w-full rounded-xl dark:block"
          fetchPriority="high"
        />
        <img
          src="/screens/visao-geral-claro.webp"
          alt="Visão geral do app NEXUS no tema claro"
          width={1440}
          height={900}
          className="block w-full rounded-xl dark:hidden"
        />
      </motion.div>
    </div>
  );
}

export function Hero() {
  return (
    <section className="relative overflow-hidden pt-36 pb-10 sm:pt-44">
      <Aurora />
      <div aria-hidden="true" className="bg-grid absolute inset-0" />
      <div className="relative mx-auto max-w-6xl px-5 text-center">
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6, ease: EASE }}
        >
          <Badge>
            <span className="size-1.5 rounded-full bg-success shadow-[0_0_8px_var(--success)]" />
            <ShinyText>v{site.version} · local-first · código aberto</ShinyText>
          </Badge>
        </motion.div>

        <h1 className="mx-auto mt-7 max-w-4xl font-display text-[clamp(2.6rem,6.4vw,5rem)] leading-[1.02] font-bold">
          <BlurText text="O contexto do seu código," className="block" />{" "}
          <BlurText
            text="na mão dos seus agentes."
            className="text-gradient block pb-1"
            delay={0.25}
          />
        </h1>

        <motion.p
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.7, ease: EASE, delay: 0.55 }}
          className="mx-auto mt-6 max-w-2xl text-lg leading-relaxed text-muted-foreground sm:text-xl"
        >
          A NEXUS indexa seus projetos{" "}
          <strong className="font-medium text-foreground">no seu computador</strong> e dá ao Claude
          Code — e a qualquer agente com MCP — busca com evidências, Context Packs e a memória do
          projeto.
        </motion.p>

        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.7, ease: EASE, delay: 0.7 }}
          className="mt-9 flex flex-wrap items-center justify-center gap-3"
          id="download"
        >
          <Magnet>
            <Button size="lg" asChild>
              <a href={downloadHref || "#como-funciona"}>
                <Download />
                Baixar para Windows
              </a>
            </Button>
          </Magnet>
          {site.githubUrl ? (
            <Button size="lg" variant="outline" asChild>
              <a href={site.githubUrl} rel="noopener">
                <GithubIcon />
                Ver no GitHub
              </a>
            </Button>
          ) : (
            <Button size="lg" variant="outline" asChild>
              <a href="#como-funciona">
                Como funciona
                <ArrowRight />
              </a>
            </Button>
          )}
        </motion.div>

        <motion.ul
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.8, delay: 0.9 }}
          className="mt-7 flex flex-wrap items-center justify-center gap-x-6 gap-y-2 text-sm text-muted-foreground"
        >
          <li className="flex items-center gap-1.5">
            <Lock className="size-4" aria-hidden="true" /> Nada sai da sua máquina
          </li>
          <li className="flex items-center gap-1.5">
            <EyeOff className="size-4" aria-hidden="true" /> Sem telemetria
          </li>
          <li className="flex items-center gap-1.5">
            <Monitor className="size-4" aria-hidden="true" /> App para Windows
          </li>
        </motion.ul>

        <ProductShot />
      </div>
    </section>
  );
}
