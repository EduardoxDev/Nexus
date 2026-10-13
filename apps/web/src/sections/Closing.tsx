import { ArrowRight, Download, Heart } from "lucide-react";

import { Aurora, Magnet, Reveal } from "@/components/fx";
import { GithubIcon } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { downloadHref, site } from "@/site";

import { Logo } from "./Nav";

export function FinalCta() {
  return (
    <section className="relative py-16 sm:py-20">
      <div className="mx-auto max-w-6xl px-5">
        <Reveal className="relative overflow-hidden rounded-3xl border border-border bg-card px-6 py-20 text-center">
          <Aurora />
          <div aria-hidden="true" className="bg-grid absolute inset-0" />
          <div className="relative">
            <Logo className="mx-auto size-16 rounded-2xl" />
            <h2 className="mx-auto mt-7 max-w-2xl font-display text-4xl font-bold sm:text-5xl">
              Dê ao seu agente a memória que faltava.
            </h2>
            <p className="mx-auto mt-4 max-w-xl text-lg text-muted-foreground">{site.slogan}</p>
            <div className="mt-9 flex flex-wrap items-center justify-center gap-3">
              <Magnet>
                <Button size="lg" asChild>
                  <a href={downloadHref || "#como-funciona"}>
                    <Download />
                    Baixar a NEXUS
                  </a>
                </Button>
              </Magnet>
              {site.githubUrl ? (
                <Button size="lg" variant="outline" asChild>
                  <a href={site.githubUrl} rel="noopener">
                    <GithubIcon />
                    Dar uma estrela
                  </a>
                </Button>
              ) : (
                <Button size="lg" variant="outline" asChild>
                  <a href="#faq">
                    Perguntas frequentes
                    <ArrowRight />
                  </a>
                </Button>
              )}
            </div>
          </div>
        </Reveal>
      </div>
    </section>
  );
}

export function Footer() {
  const year = 2026;
  return (
    <footer className="border-t border-border py-10">
      <div className="mx-auto flex max-w-6xl flex-col items-center justify-between gap-4 px-5 text-sm text-muted-foreground sm:flex-row">
        <div className="flex items-center gap-2.5">
          <Logo className="size-6" />
          <span className="font-display font-bold tracking-[0.14em] text-foreground">
            {site.name}
          </span>
          <span>· {site.tagline}</span>
        </div>
        <p className="flex items-center gap-1.5">
          © {year} · Licença MIT · Feito com
          <Heart
            className="size-3.5 fill-[oklch(0.65_0.2_15)] text-[oklch(0.65_0.2_15)]"
            aria-label="amor"
          />
          {site.maker.name ? <>por {site.maker.name}</> : <>no Brasil</>}
        </p>
      </div>
    </footer>
  );
}
