import { MotionConfig } from "motion/react";

import { FinalCta, Footer } from "@/sections/Closing";
import { Faq } from "@/sections/Faq";
import { Features } from "@/sections/Features";
import { Hero } from "@/sections/Hero";
import { HowItWorks } from "@/sections/HowItWorks";
import { Nav } from "@/sections/Nav";
import { Numbers } from "@/sections/Numbers";
import { Privacy } from "@/sections/Privacy";
import { Problem } from "@/sections/Problem";
import { Showcase } from "@/sections/Showcase";
import { Story } from "@/sections/Story";

export function App() {
  return (
    <MotionConfig reducedMotion="user">
      <a
        href="#conteudo"
        className="sr-only z-[60] rounded-md bg-primary px-4 py-2 text-primary-foreground focus:not-sr-only focus:fixed focus:top-3 focus:left-3"
      >
        Pular para o conteúdo
      </a>
      <Nav />
      <main id="conteudo">
        <Hero />
        <Problem />
        <Features />
        <HowItWorks />
        <Showcase />
        <Numbers />
        <Privacy />
        <Story />
        <Faq />
        <FinalCta />
      </main>
      <Footer />
    </MotionConfig>
  );
}
