/**
 * Botões de chamada do site: o principal com a luz correndo na borda (StarBorder),
 * o secundário com um reflexo que passa no hover (GlareHover). Ambos do React Bits.
 */
import { Download } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";

import GlareHover from "@/components/GlareHover";
import StarBorder from "@/components/StarBorder";
import { useTheme } from "@/lib/useTheme";
import { downloadHref } from "@/site";

const SIZE = "h-12 px-6 text-[15px] font-medium";

type System = "Windows" | "macOS" | "Linux";

/** Sistema de quem visita (só no navegador; o HTML pré-renderizado sai com "Windows"). */
function useSystem(): System {
  const [system, setSystem] = useState<System>("Windows");
  useEffect(() => {
    const p = `${navigator.platform} ${navigator.userAgent}`;
    if (/Mac|iPhone|iPad/i.test(p)) setSystem("macOS");
    else if (/Linux|X11/i.test(p) && !/Android/i.test(p)) setSystem("Linux");
  }, []);
  return system;
}

/** Leva à última release publicada, que traz os pacotes de todos os sistemas. */
export function DownloadButton({ label }: { label?: string }) {
  const system = useSystem();
  label ??= `Baixar para ${system}`;
  return (
    <StarBorder
      as="a"
      href={downloadHref || "#comecar"}
      radius={12}
      thickness={1}
      duration={5}
      glow={0.8}
      sparkle
      hover="lap"
      color="#ffffff"
      trailColor="#9fb4ff"
      backgroundColor="#4f6fff"
      borderColor="rgba(255, 255, 255, 0.22)"
      textColor="#ffffff"
      className={`${SIZE} shadow-[0_10px_30px_-10px_rgb(79_111_255/0.7)] transition-[filter] hover:brightness-110`}
    >
      <Download className="size-[18px]" />
      {label}
    </StarBorder>
  );
}

export function GhostButton({
  href,
  children,
  rel,
}: {
  href: string;
  children: ReactNode;
  rel?: string;
}) {
  const theme = useTheme();
  return (
    <GlareHover
      borderRadius="12px"
      background="transparent"
      glareColor={theme === "light" ? "#4f6fff" : "#ffffff"}
      glareOpacity={theme === "light" ? 0.18 : 0.28}
      glareSize={120}
      transitionDuration={700}
      className="inline-flex"
    >
      <a
        href={href}
        {...(rel ? { rel } : {})}
        className={`${SIZE} inline-flex items-center gap-2 rounded-xl border border-border bg-secondary/60 text-foreground transition-colors hover:bg-secondary`}
      >
        {children}
      </a>
    </GlareHover>
  );
}
