import { Check, Copy } from "lucide-react";
import { useState } from "react";

import AnimatedContent from "@/components/AnimatedContent";
import CountUp from "@/components/CountUp";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

const COMMAND = "claude mcp add nexus -- nexus mcp serve";

/** É de fato uma sequência: os números marcam a ordem. */
const STEPS = [
  {
    title: "Instale e escolha uma pasta",
    body: "O instalador não pede administrador. Você aponta para o projeto; a Nexus nunca lê nada fora dele.",
  },
  {
    title: "Espere alguns segundos",
    body: "Ela indexa sozinha e deixa de fora senhas, chaves e binários. Depois, acompanha o que você salvar.",
  },
  {
    title: "Conecte o Claude Code",
    body: "Rode o comando abaixo uma vez. A tela Integrações do app mostra a versão com os seus caminhos.",
  },
];

/** Medições reais (docs/benchmarks.md e docs/evaluation.md). */
const STATS = [
  { to: 3, suffix: " s", prefix: "~", label: "para indexar 10 mil arquivos" },
  { to: 37, suffix: " ms", label: "por busca, na mediana" },
  { to: 13, suffix: "", label: "ferramentas para agentes (MCP)" },
];

function CopyCommand() {
  const [copied, setCopied] = useState(false);
  const copy = () => {
    void navigator.clipboard
      .writeText(COMMAND)
      .then(() => {
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1600);
      })
      .catch(() => setCopied(false));
  };
  return (
    <div className="flex items-center gap-3 rounded-2xl border border-border bg-codigo py-2.5 pr-2.5 pl-5 text-codigo-texto">
      <span aria-hidden="true" className="font-codigo text-[#7ee2a8]">
        $
      </span>
      <code className="font-codigo min-w-0 flex-1 overflow-x-auto text-[15px] whitespace-nowrap">
        {COMMAND}
      </code>
      <Button size="sm" variant="secondary" onClick={copy}>
        {copied ? <Check /> : <Copy />}
        {copied ? "Copiado" : "Copiar"}
      </Button>
    </div>
  );
}

export function GetStarted() {
  return (
    <section id="comecar" className="scroll-mt-20">
      <div className="mx-auto max-w-6xl px-5 py-24">
        <AnimatedContent distance={40} duration={0.8}>
          <h2 className="font-titulo text-[clamp(2rem,3.8vw,3rem)] leading-[1.08]">
            Em três passos, sem criar conta
          </h2>
        </AnimatedContent>
        <ol className="mt-12 grid gap-5 md:grid-cols-3">
          {STEPS.map((s, i) => (
            <li key={s.title}>
              <AnimatedContent distance={50} delay={i * 0.12} duration={0.8} className="h-full">
                <Card className="h-full gap-3 rounded-2xl">
                  <CardHeader>
                    <span
                      aria-hidden="true"
                      className="font-titulo flex size-10 items-center justify-center rounded-full bg-primary text-lg text-primary-foreground"
                    >
                      {i + 1}
                    </span>
                    <CardTitle className="mt-3 font-titulo text-xl">{s.title}</CardTitle>
                    <CardDescription className="text-base leading-relaxed">
                      {s.body}
                    </CardDescription>
                  </CardHeader>
                </Card>
              </AnimatedContent>
            </li>
          ))}
        </ol>
        <AnimatedContent distance={30} duration={0.8} className="mt-8 max-w-2xl">
          <CopyCommand />
        </AnimatedContent>

        <dl className="mt-16 grid gap-5 sm:grid-cols-3">
          {STATS.map((s) => (
            <Card key={s.label} className="rounded-2xl">
              <CardContent>
                <dd className="font-titulo text-5xl text-primary">
                  {s.prefix}
                  <CountUp to={s.to} duration={1.6} />
                  {s.suffix}
                </dd>
                <dt className="mt-2 text-muted-foreground">{s.label}</dt>
              </CardContent>
            </Card>
          ))}
        </dl>
      </div>
    </section>
  );
}
