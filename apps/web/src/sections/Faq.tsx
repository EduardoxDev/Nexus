import { SectionHeader } from "@/components/SectionHeader";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";

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
    a: "Com qualquer um que fale MCP. A tela Integrações traz a configuração pronta para Claude Code, Codex, Cursor, VS Code e Claude Desktop, com os caminhos da sua instalação. Também dá para usar sem agente: o app e a linha de comando fazem busca e Context Packs.",
  },
  {
    q: "Quais linguagens ela entende?",
    a: "A busca funciona em qualquer arquivo de texto. A análise de declarações e imports cobre TypeScript, JavaScript, Python, Rust, Go, Java, C#, Kotlin, PHP, Ruby, C e C++.",
  },
  {
    q: "Funciona no Mac ou no Linux?",
    a: "Sim. Há instalador para Windows, .deb e .AppImage para Linux e .dmg para macOS (Apple Silicon e Intel), todos com a linha de comando incluída.",
  },
  {
    q: "Ela se atualiza sozinha?",
    a: "Só se você deixar. Na primeira vez que abre, o app pergunta se pode verificar versões novas no GitHub (no máximo uma vez por dia). A instalação só acontece quando você clica, e cada atualização é conferida por assinatura antes de instalar. Vale para o instalador do Windows, o .AppImage do Linux e o macOS; quem instalou pelo .deb atualiza baixando o .deb novo.",
  },
  {
    q: "O Windows avisou que o instalador não é reconhecido. É seguro?",
    a: "O instalador ainda não tem assinatura de código, por isso o aviso (no Mac, o Gatekeeper também avisa). O código é aberto e cada versão publicada traz o SHA-256 dos arquivos para você conferir.",
  },
];

export function Faq() {
  return (
    <section id="perguntas" className="scroll-mt-20">
      <div className="mx-auto grid max-w-6xl gap-10 px-5 py-24 lg:grid-cols-[0.8fr_1.2fr]">
        <SectionHeader title="Perguntas que sempre aparecem" />
        <div className="rounded-[20px] border border-border bg-card px-6 py-2">
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
        </div>
      </div>
    </section>
  );
}
