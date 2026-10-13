/**
 * Uma resposta real da NEXUS (fixture saas-app): a pergunta do agente, o trecho citado com as
 * linhas exatas grifadas e o motivo. Ao abrir a página, as linhas são grifadas uma a uma — o único
 * movimento automático do site.
 */
import { Check } from "lucide-react";
import { motion, useReducedMotion } from "motion/react";

import { cn } from "@/lib/cn";

const CODE: { n: number; text: string; cited?: boolean }[] = [
  { n: 1, text: 'import type { Cents } from "../utils/money";' },
  { n: 2, text: "" },
  { n: 3, text: "/** Frete por faixa de CEP; acima de R$ 300 é grátis. */", cited: true },
  { n: 4, text: "export function shippingCost(zip: string, subtotal: Cents) {", cited: true },
  { n: 5, text: "  if (subtotal >= 30_000) return 0;" },
  { n: 6, text: "  const region = Number(zip.slice(0, 1));" },
  { n: 7, text: "  return region <= 3 ? 1_500 : 2_900;" },
  { n: 8, text: "}" },
];

const REASONS = [
  "o caminho fala de frete (shipping)",
  "declara shippingCost na linha 4",
  "o arquivo no disco é o mesmo que foi indexado",
];

export function EvidenceCard({ className }: { className?: string }) {
  const reduce = useReducedMotion();
  let cited = 0;
  return (
    <figure
      className={cn(
        "relative rounded-[22px] border border-border bg-card p-5 shadow-[0_1px_0_#fff_inset,0_24px_60px_-28px_rgba(35,72,206,0.35)] sm:p-6",
        className,
      )}
    >
      <figcaption className="sr-only">
        Exemplo: o Claude Code pergunta onde o frete é calculado e a NEXUS responde citando o
        arquivo src/checkout/shipping.ts, linhas 3 e 4.
      </figcaption>

      {/* Espaço à direita para a nuvem que espia o cartão. */}
      <p className="pr-28 text-[15px] text-muted-foreground sm:pr-32">
        O Claude Code perguntou:{" "}
        <span className="font-semibold text-foreground">“Onde o frete é calculado?”</span>
      </p>

      <div className="mt-4 flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <p className="font-codigo text-[14px] font-semibold text-primary">
          src/checkout/shipping.ts
        </p>
        <p className="flex items-center gap-1 text-[13px] text-muted-foreground">
          <Check className="size-3.5 text-[#1f9d55]" aria-hidden="true" />
          linhas 3 e 4, conferidas agora
        </p>
      </div>

      <pre
        className="font-codigo mt-3 overflow-x-auto rounded-xl bg-codigo py-3 text-[12.5px] leading-[1.7] text-codigo-texto"
        aria-label="Trecho citado de src/checkout/shipping.ts"
      >
        {CODE.map((l) => {
          const order = l.cited ? cited++ : -1;
          return (
            <div key={l.n} className="relative flex px-4">
              {l.cited && (
                <motion.span
                  aria-hidden="true"
                  className="absolute inset-y-0 left-0 border-l-[3px] border-[#ffe27a] bg-[#ffe27a]/25"
                  initial={reduce ? { width: "100%" } : { width: "0%" }}
                  animate={{ width: "100%" }}
                  transition={{
                    duration: 0.55,
                    delay: 0.9 + order * 0.45,
                    ease: [0.65, 0, 0.35, 1],
                  }}
                />
              )}
              <span
                className={cn(
                  "relative w-7 shrink-0 select-none text-right",
                  l.cited ? "text-[#ffe27a]" : "text-[#5d6b99]",
                )}
              >
                {l.n}
              </span>
              <code className="relative ml-4 whitespace-pre">{l.text || " "}</code>
            </div>
          );
        })}
      </pre>

      <div className="mt-4 text-[14px] leading-relaxed">
        <p className="text-muted-foreground">Por que este trecho:</p>
        <ul className="mt-1.5 space-y-1">
          {REASONS.map((r) => (
            <li key={r} className="flex gap-2">
              <span
                aria-hidden="true"
                className="mt-[0.55em] size-1.5 shrink-0 rounded-full bg-primary"
              />
              {r}
            </li>
          ))}
        </ul>
      </div>
    </figure>
  );
}
