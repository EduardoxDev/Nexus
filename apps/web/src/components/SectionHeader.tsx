import type { ReactNode } from "react";

import AnimatedContent from "@/components/AnimatedContent";

/** Título de seção: peso médio, apertado; `accent` vira uma segunda linha em cinza. */
export function SectionHeader({
  title,
  accent,
  children,
}: {
  title: string;
  accent?: string;
  children?: ReactNode;
}) {
  return (
    <AnimatedContent distance={30} duration={0.8}>
      <h2 className="max-w-2xl font-titulo text-[clamp(2rem,3.6vw,2.75rem)] leading-[1.1]">
        {title}
        {accent && (
          <>
            {" "}
            <span className="block text-muted-foreground">{accent}</span>
          </>
        )}
      </h2>
      {children && (
        <p className="mt-5 max-w-xl text-[17px] leading-relaxed text-muted-foreground">
          {children}
        </p>
      )}
    </AnimatedContent>
  );
}
