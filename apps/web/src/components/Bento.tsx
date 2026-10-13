/**
 * O MagicBento (React Bits) com a configuração aprovada na seção "O que ela faz por você":
 * holofote, brilho na borda, partículas e o azul da marca. Todas as seções usam este mesmo visual.
 */
import MagicBento, { type MagicBentoItem } from "@/components/MagicBento";
import { useTheme } from "@/lib/useTheme";

export type { MagicBentoItem as BentoItem };

export function Bento({
  items,
  columns = 4,
  rowHeight = 200,
  stars = false,
  className,
}: {
  items: MagicBentoItem[];
  columns?: number;
  rowHeight?: number;
  /** Partículas animadas em cada card: bonitas, mas custam a cada quadro — só onde fazem diferença. */
  stars?: boolean;
  className?: string;
}) {
  const theme = useTheme();
  return (
    <MagicBento
      items={items}
      columns={columns}
      rowHeight={rowHeight}
      gap={14}
      radius={20}
      theme={theme}
      glowColor="79, 111, 255"
      enableStars={stars}
      enableSpotlight
      enableBorderGlow
      enableTilt={false}
      enableMagnetism={false}
      clickEffect
      particleCount={10}
      spotlightRadius={320}
      textAutoHide={false}
      {...(className ? { className } : {})}
    />
  );
}

/** Captura real do app dentro de um card. */
export function Shot({
  src,
  alt,
  className = "h-44 sm:h-52",
}: {
  src: string;
  alt: string;
  className?: string;
}) {
  return (
    <img
      src={src}
      alt={alt}
      loading="lazy"
      width={1440}
      height={900}
      className={`w-full rounded-lg border border-white/10 object-cover object-top ${className}`}
    />
  );
}
