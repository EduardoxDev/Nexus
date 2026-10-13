import type { ReactNode } from "react";

import AnimatedContent from "@/components/AnimatedContent";

/** Título de seção no estilo aprovado: grande, alinhado à esquerda, com uma linha de apoio. */
export function SectionHeader({ title, children }: { title: string; children?: ReactNode }) {
  return (
    <AnimatedContent distance={40} duration={0.8}>
      <h2 className="max-w-2xl font-titulo text-[clamp(2rem,3.8vw,3rem)] leading-[1.08]">
        {title}
      </h2>
      {children && <p className="mt-4 max-w-xl text-lg text-muted-foreground">{children}</p>}
    </AnimatedContent>
  );
}
