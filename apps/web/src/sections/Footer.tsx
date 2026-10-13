import Magnet from "@/components/Magnet";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { downloadHref, site } from "@/site";

import { Logo } from "./Nav";

export function Footer() {
  return (
    <footer>
      <div className="mx-auto max-w-6xl px-5 pt-20 pb-12">
        <div className="flex flex-col gap-8 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <h2 className="max-w-md font-titulo text-[clamp(1.9rem,3.4vw,2.6rem)] leading-[1.08]">
              Dê ao seu agente a memória que faltava.
            </h2>
            <p className="mt-3 text-muted-foreground">{site.slogan}</p>
          </div>
          <Magnet padding={50} magnetStrength={4}>
            <Button size="lg" className="h-12 rounded-xl px-6 text-base" asChild>
              <a href={downloadHref || "#comecar"}>Baixar para Windows</a>
            </Button>
          </Magnet>
        </div>
        <Separator className="my-10" />
        <div className="flex flex-col gap-3 text-sm text-muted-foreground sm:flex-row sm:items-center sm:justify-between">
          <div className="flex items-center gap-2.5">
            <Logo className="size-7" />
            <span className="font-titulo text-base text-foreground">Nexus</span>
          </div>
          <p>Licença MIT{site.maker.name ? `, feita por ${site.maker.name}` : ""}. 2026.</p>
        </div>
      </div>
    </footer>
  );
}
