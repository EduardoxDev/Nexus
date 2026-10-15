import { MotionConfig } from "motion/react";

import { TooltipProvider } from "@/components/ui/tooltip";
import { AppShot } from "@/sections/AppShot";
import { Faq } from "@/sections/Faq";
import { Footer } from "@/sections/Footer";
import { GetStarted } from "@/sections/GetStarted";
import { Hero } from "@/sections/Hero";
import { Nav } from "@/sections/Nav";
import { Privacy } from "@/sections/Privacy";
import { DocPage } from "@/sections/DocPage";
import { WhatItDoes } from "@/sections/WhatItDoes";
import { DOCS } from "@/routes";

/** `path` = página do site sem a base (`/`, `/docs/instalar/`…). */
export function App({ path = "/" }: { path?: string }) {
  const doc = DOCS.find((d) => d.path === path);
  return (
    <MotionConfig reducedMotion="user">
      <TooltipProvider delayDuration={200}>
        <a
          href="#conteudo"
          className="sr-only z-[60] rounded-md bg-primary px-4 py-2 text-primary-foreground focus:not-sr-only focus:fixed focus:top-3 focus:left-3"
        >
          Pular para o conteúdo
        </a>
        {doc ? (
          <DocPage doc={doc} />
        ) : (
          <>
            <Nav />
            <main id="conteudo">
              <Hero />
              <WhatItDoes />
              <GetStarted />
              <AppShot />
              <Privacy />
              <Faq />
            </main>
            <Footer />
          </>
        )}
      </TooltipProvider>
    </MotionConfig>
  );
}
