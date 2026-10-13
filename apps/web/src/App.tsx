import { MotionConfig } from "motion/react";

import { TooltipProvider } from "@/components/ui/tooltip";
import { AppShot } from "@/sections/AppShot";
import { Faq } from "@/sections/Faq";
import { Footer } from "@/sections/Footer";
import { GetStarted } from "@/sections/GetStarted";
import { Hero } from "@/sections/Hero";
import { Nav } from "@/sections/Nav";
import { Privacy } from "@/sections/Privacy";
import { WhatItDoes } from "@/sections/WhatItDoes";

export function App() {
  return (
    <MotionConfig reducedMotion="user">
      <TooltipProvider delayDuration={200}>
        <a
          href="#conteudo"
          className="sr-only z-[60] rounded-md bg-primary px-4 py-2 text-primary-foreground focus:not-sr-only focus:fixed focus:top-3 focus:left-3"
        >
          Pular para o conteúdo
        </a>
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
      </TooltipProvider>
    </MotionConfig>
  );
}
