import { Download } from "lucide-react";

import { Bento } from "@/components/Bento";
import Magnet from "@/components/Magnet";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { downloadHref, site } from "@/site";

import { Logo } from "./Nav";

function FinalCall() {
  return (
    <div className="flex h-full flex-col items-start justify-center gap-6 sm:flex-row sm:items-center sm:justify-between">
      <div className="flex items-center gap-5">
        <Logo className="size-16 rounded-2xl" />
        <div>
          <p className="font-titulo text-[clamp(1.6rem,3vw,2.3rem)] leading-tight text-foreground">
            Dê ao seu agente a memória que faltava.
          </p>
          <p className="mt-1 text-muted-foreground">{site.slogan}</p>
        </div>
      </div>
      <Magnet padding={50} magnetStrength={4}>
        <Button size="lg" className="h-12 rounded-xl px-6 text-base" asChild>
          <a href={downloadHref || "#comecar"}>
            <Download className="size-5" />
            Baixar para Windows
          </a>
        </Button>
      </Magnet>
    </div>
  );
}

export function Footer() {
  return (
    <footer>
      <div className="mx-auto max-w-6xl px-5 pt-8 pb-12">
        <Bento columns={1} rowHeight={190} items={[{ content: <FinalCall /> }]} />
        <Separator className="my-10" />
        <div className="flex flex-col gap-3 text-sm text-muted-foreground sm:flex-row sm:items-center sm:justify-between">
          <div className="flex items-center gap-2.5">
            <Logo className="size-7" />
            <span className="font-titulo text-base text-foreground">Nexus</span>
          </div>
          <p>
            Licença MIT{site.maker.name ? `, feita com carinho por ${site.maker.name}` : ""}. 2026.
          </p>
        </div>
      </div>
    </footer>
  );
}
