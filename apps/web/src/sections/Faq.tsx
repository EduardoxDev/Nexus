import { Reveal } from "@/components/fx";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";

const FAQ = [
  {
    q: "A NEXUS é grátis?",
    a: "Sim. É código aberto, sob a licença MIT. Não há conta, plano pago nem limite de uso.",
  },
  {
    q: "Meu código vai para algum servidor?",
    a: "Não. A NEXUS não tem servidor e não faz telemetria: o índice e a memória ficam num banco local. Se você usa um agente na nuvem (como o Claude Code), ele envia ao provedor os trechos que ler — exatamente como já faz quando abre seus arquivos.",
  },
  {
    q: "Com quais agentes ela funciona?",
    a: "Com qualquer cliente MCP: o Claude Code em um comando e outros clientes pelo arquivo de configuração mcpServers, que a tela Integrações gera pronto. Também dá para usar sem agente: o app e a CLI fazem busca e Context Packs.",
  },
  {
    q: "Quais linguagens ela entende?",
    a: "A busca funciona em qualquer arquivo de texto. A análise de declarações e imports — que alimenta o grafo de dependências — cobre TypeScript, JavaScript, Python e Rust.",
  },
  {
    q: "Funciona no Mac ou no Linux?",
    a: "O app é para Windows por enquanto. O núcleo e a CLI são em Rust e multiplataforma; versões para outros sistemas estão no radar.",
  },
  {
    q: "O Windows avisou que o instalador não é reconhecido. É seguro?",
    a: "O instalador ainda não tem assinatura de código (o certificado está pendente), por isso o SmartScreen alerta. O código é aberto e cada release traz o SHA-256 do instalador para você conferir.",
  },
];

export function Faq() {
  return (
    <section id="faq" className="relative scroll-mt-20 py-16 sm:py-20">
      <div className="mx-auto max-w-3xl px-5">
        <Reveal className="text-center">
          <p className="text-sm font-semibold tracking-[0.16em] text-primary uppercase">
            Perguntas
          </p>
          <h2 className="mt-3 font-display text-4xl font-bold sm:text-5xl">Perguntas frequentes</h2>
        </Reveal>
        <Reveal className="mt-12 rounded-2xl border border-border bg-card px-6">
          <Accordion type="single" collapsible>
            {FAQ.map((f) => (
              <AccordionItem key={f.q} value={f.q}>
                <AccordionTrigger>{f.q}</AccordionTrigger>
                <AccordionContent>{f.a}</AccordionContent>
              </AccordionItem>
            ))}
          </Accordion>
        </Reveal>
      </div>
    </section>
  );
}
