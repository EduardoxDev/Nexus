import { Check, Copy, FolderGit2, Terminal, Zap } from "lucide-react";
import { useState } from "react";

import { Reveal } from "@/components/fx";
import { Button } from "@/components/ui/button";

const COMMAND = "claude mcp add nexus -- nexus mcp serve";

const STEPS = [
  {
    icon: FolderGit2,
    title: "Autorize uma pasta",
    body: "Você escolhe os projetos. A NEXUS recusa a pasta pessoal inteira e nunca segue links para fora.",
  },
  {
    icon: Zap,
    title: "Ela indexa sozinha",
    body: "Segredos e binários ficam de fora. Depois, só o que muda é reprocessado — em menos de meio segundo.",
  },
  {
    icon: Terminal,
    title: "Conecte o seu agente",
    body: "Um comando e o Claude Code passa a consultar a NEXUS. A tela Integrações já traz tudo pronto.",
  },
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
    <div className="flex items-center gap-3 rounded-xl border border-border bg-[oklch(0.16_0.012_265)] py-2 pr-2 pl-4 font-mono text-sm text-[oklch(0.92_0.01_265)] shadow-lg">
      <span className="text-[oklch(0.65_0.15_155)] select-none" aria-hidden="true">
        $
      </span>
      <code className="min-w-0 flex-1 overflow-x-auto whitespace-nowrap">{COMMAND}</code>
      <Button
        size="sm"
        variant="ghost"
        onClick={copy}
        className="text-[oklch(0.8_0.01_265)] hover:bg-white/10 hover:text-white"
        aria-label="Copiar comando"
      >
        {copied ? <Check /> : <Copy />}
        {copied ? "Copiado" : "Copiar"}
      </Button>
    </div>
  );
}

export function HowItWorks() {
  return (
    <section id="como-funciona" className="relative scroll-mt-20 py-16 sm:py-20">
      <div className="mx-auto max-w-6xl px-5">
        <Reveal className="mx-auto max-w-3xl text-center">
          <p className="text-sm font-semibold tracking-[0.16em] text-primary uppercase">
            Como funciona
          </p>
          <h2 className="mt-3 font-display text-4xl font-bold sm:text-5xl">
            Três passos. Nenhuma conta.
          </h2>
          <p className="mt-4 text-lg text-muted-foreground">
            Sem cadastro, sem nuvem, sem chave de API. Instale, aponte para o projeto e pronto.
          </p>
        </Reveal>

        <div className="relative mt-16">
          <div
            aria-hidden="true"
            className="absolute top-6 right-[16%] left-[16%] hidden h-px bg-gradient-to-r from-transparent via-border to-transparent md:block"
          />
          <ol className="relative grid gap-6 md:grid-cols-3">
            {STEPS.map((s, i) => (
              <li key={s.title} className="text-center">
                <Reveal delay={i * 0.12}>
                  <span className="relative mx-auto flex size-12 items-center justify-center rounded-2xl border border-border bg-card shadow-sm">
                    <s.icon className="size-5 text-primary" aria-hidden="true" />
                    <span className="absolute -top-2 -right-2 flex size-5 items-center justify-center rounded-full bg-primary text-[11px] font-bold text-primary-foreground">
                      {i + 1}
                    </span>
                  </span>
                  <h3 className="mt-5 font-display text-lg font-semibold">{s.title}</h3>
                  <p className="mx-auto mt-2 max-w-xs text-[15px] leading-relaxed text-muted-foreground">
                    {s.body}
                  </p>
                </Reveal>
              </li>
            ))}
          </ol>
        </div>

        <Reveal className="mx-auto mt-14 max-w-2xl">
          <CopyCommand />
          <p className="mt-3 text-center text-xs text-muted-foreground">
            O instalador já traz a CLI <code className="font-mono">nexus</code>. O comando completo,
            com o caminho certo, aparece na tela Integrações do app.
          </p>
        </Reveal>
      </div>
    </section>
  );
}
