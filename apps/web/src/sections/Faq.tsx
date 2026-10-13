import AnimatedContent from "@/components/AnimatedContent";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Card } from "@/components/ui/card";

const FAQ = [
  {
    q: "É grátis?",
    a: "Sim. É código aberto, com licença MIT. Não tem conta, plano pago nem limite de uso.",
  },
  {
    q: "Meu código vai para algum servidor?",
    a: "Não. A Nexus não tem servidor nem coleta dados de uso. Se você usa um agente na nuvem, como o Claude Code, ele envia ao provedor os trechos que ler — exatamente como já faz quando abre seus arquivos.",
  },
  {
    q: "Funciona com quais agentes?",
    a: "Com qualquer um que fale MCP. O Claude Code se conecta com um comando; para os outros, a tela Integrações gera a configuração pronta. Também dá para usar sem agente: o app e a linha de comando fazem busca e Context Packs.",
  },
  {
    q: "Quais linguagens ela entende?",
    a: "A busca funciona em qualquer arquivo de texto. A análise de declarações e imports cobre TypeScript, JavaScript, Python e Rust.",
  },
  {
    q: "Funciona no Mac ou no Linux?",
    a: "O app é para Windows, por enquanto. O núcleo e a linha de comando são em Rust e funcionam em outros sistemas; versões completas estão nos planos.",
  },
  {
    q: "O Windows avisou que o instalador não é reconhecido. É seguro?",
    a: "O instalador ainda não tem assinatura de código, por isso o aviso. O código é aberto e cada versão publicada traz o SHA-256 do instalador para você conferir.",
  },
];

export function Faq() {
  return (
    <section id="perguntas" className="scroll-mt-20">
      <div className="mx-auto grid max-w-6xl gap-10 px-5 py-24 lg:grid-cols-[0.8fr_1.2fr]">
        <AnimatedContent distance={40} duration={0.8}>
          <h2 className="font-titulo text-[clamp(2rem,3.8vw,3rem)] leading-[1.08]">
            Perguntas que sempre aparecem
          </h2>
        </AnimatedContent>
        <Card className="rounded-2xl px-6 py-2">
          <Accordion type="single" collapsible>
            {FAQ.map((f) => (
              <AccordionItem key={f.q} value={f.q}>
                <AccordionTrigger className="font-titulo text-[1.05rem]">{f.q}</AccordionTrigger>
                <AccordionContent className="text-base leading-relaxed text-muted-foreground">
                  {f.a}
                </AccordionContent>
              </AccordionItem>
            ))}
          </Accordion>
        </Card>
      </div>
    </section>
  );
}
