/**
 * Efeitos visuais no estilo React Bits (reactbits.dev), escritos aqui sem dependências extras.
 * Todos respeitam `prefers-reduced-motion` (CSS global e `useReducedMotion`).
 */
import {
  animate,
  motion,
  useInView,
  useMotionValue,
  useReducedMotion,
  useSpring,
  type HTMLMotionProps,
} from "motion/react";
import {
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type MouseEvent,
  type ReactNode,
} from "react";

import { cn } from "@/lib/cn";

const EASE = [0.22, 1, 0.36, 1] as const;

/**
 * Aurora: manchas de luz que se movem devagar atrás do conteúdo. O degradê radial já é suave, então
 * não há `filter: blur` (caro a cada quadro); cada mancha anima só `transform`, numa camada própria.
 */
export function Aurora({ className }: { className?: string }) {
  return (
    <div
      aria-hidden="true"
      className={cn(
        "pointer-events-none absolute inset-0 overflow-hidden [contain:paint]",
        className,
      )}
    >
      <div className="absolute -top-[30%] left-[10%] h-[70%] w-[55%] animate-aurora rounded-full bg-[radial-gradient(closest-side,var(--glow),transparent)] will-change-transform" />
      <div className="absolute -top-[20%] right-[5%] h-[60%] w-[45%] animate-aurora rounded-full bg-[radial-gradient(closest-side,var(--glow-2),transparent)] will-change-transform [animation-delay:-6s]" />
      <div className="absolute top-[20%] left-[35%] h-[50%] w-[40%] animate-aurora rounded-full bg-[radial-gradient(closest-side,var(--glow),transparent)] opacity-60 will-change-transform [animation-delay:-12s]" />
    </div>
  );
}

/** Card com holofote que segue o mouse (SpotlightCard). */
export function SpotlightCard({
  children,
  className,
  color = "oklch(0.62 0.2 266 / 0.14)",
}: {
  children: ReactNode;
  className?: string;
  color?: string;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const onMove = (e: MouseEvent<HTMLDivElement>) => {
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    el.style.setProperty("--mx", `${e.clientX - r.left}px`);
    el.style.setProperty("--my", `${e.clientY - r.top}px`);
  };
  return (
    <div
      ref={ref}
      onMouseMove={onMove}
      style={{ "--spot": color } as CSSProperties}
      className={cn(
        "group relative overflow-hidden rounded-2xl border border-border bg-card transition-colors duration-300 hover:border-primary/30",
        "before:pointer-events-none before:absolute before:inset-0 before:opacity-0 before:transition-opacity before:duration-500 hover:before:opacity-100",
        "before:bg-[radial-gradient(420px_circle_at_var(--mx,50%)_var(--my,50%),var(--spot),transparent_60%)]",
        className,
      )}
    >
      {children}
    </div>
  );
}

/** Texto com um brilho que passa (ShinyText). */
export function ShinyText({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <span
      className={cn(
        "animate-shine bg-[linear-gradient(110deg,var(--muted-foreground)_35%,var(--foreground)_50%,var(--muted-foreground)_65%)] bg-[length:200%_100%] bg-clip-text text-transparent",
        className,
      )}
    >
      {children}
    </span>
  );
}

/** Frase que entra palavra por palavra, saindo do desfoque (BlurText). */
export function BlurText({
  text,
  className,
  delay = 0,
  as = "span",
}: {
  text: string;
  className?: string;
  delay?: number;
  as?: "span" | "h1" | "h2" | "p";
}) {
  const reduce = useReducedMotion();
  const words = text.split(" ");
  const Tag = motion[as];
  return (
    <Tag className={className}>
      {/* Leitores de tela leem a frase inteira; as palavras animadas são só visuais. */}
      <span className="sr-only">{text}</span>
      {words.map((w, i) => (
        <motion.span
          key={`${w}-${i}`}
          aria-hidden="true"
          className="inline-block whitespace-pre"
          initial={reduce ? false : { opacity: 0, filter: "blur(10px)", y: 12 }}
          animate={{ opacity: 1, filter: "blur(0px)", y: 0 }}
          transition={{ duration: 0.7, ease: EASE, delay: delay + i * 0.06 }}
        >
          {w}
          {i < words.length - 1 ? " " : ""}
        </motion.span>
      ))}
    </Tag>
  );
}

/** Aparece subindo quando entra na tela. */
export function Reveal({
  children,
  className,
  delay = 0,
  ...rest
}: { children: ReactNode; className?: string; delay?: number } & Omit<
  HTMLMotionProps<"div">,
  "children"
>) {
  const reduce = useReducedMotion();
  return (
    <motion.div
      className={className}
      initial={reduce ? false : { opacity: 0, y: 24 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-80px" }}
      transition={{ duration: 0.7, ease: EASE, delay }}
      {...rest}
    >
      {children}
    </motion.div>
  );
}

/** Número que conta até o valor quando aparece na tela (CountUp). */
export function CountUp({
  to,
  decimals = 0,
  suffix = "",
  prefix = "",
  className,
}: {
  to: number;
  decimals?: number;
  suffix?: string;
  prefix?: string;
  className?: string;
}) {
  const ref = useRef<HTMLSpanElement>(null);
  const inView = useInView(ref, { once: true, margin: "-60px" });
  const reduce = useReducedMotion();
  const format = (v: number) =>
    `${prefix}${v.toLocaleString("pt-BR", { minimumFractionDigits: decimals, maximumFractionDigits: decimals })}${suffix}`;
  const [shown, setShown] = useState(format(to));

  useEffect(() => {
    if (!inView || reduce) return;
    const controls = animate(0, to, {
      duration: 1.6,
      ease: EASE,
      onUpdate: (v) => setShown(format(v)),
    });
    return () => controls.stop();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [inView, reduce, to]);

  return (
    <span ref={ref} className={cn("tabular-nums", className)}>
      {shown}
    </span>
  );
}

/** Elemento atraído levemente pelo cursor (Magnet). */
export function Magnet({ children, strength = 0.25 }: { children: ReactNode; strength?: number }) {
  const x = useMotionValue(0);
  const y = useMotionValue(0);
  const sx = useSpring(x, { stiffness: 220, damping: 18 });
  const sy = useSpring(y, { stiffness: 220, damping: 18 });
  const reduce = useReducedMotion();
  return (
    <motion.div
      className="inline-block"
      style={{ x: sx, y: sy }}
      onMouseMove={(e) => {
        if (reduce) return;
        const r = e.currentTarget.getBoundingClientRect();
        x.set((e.clientX - r.left - r.width / 2) * strength);
        y.set((e.clientY - r.top - r.height / 2) * strength);
      }}
      onMouseLeave={() => {
        x.set(0);
        y.set(0);
      }}
    >
      {children}
    </motion.div>
  );
}

/** Borda com um feixe de luz girando (BorderBeam). */
export function BorderBeam({ className }: { className?: string }) {
  return (
    <div
      aria-hidden="true"
      className={cn(
        "pointer-events-none absolute inset-0 animate-beam rounded-[inherit] p-px",
        "[background:conic-gradient(from_var(--beam-angle),transparent_0%,oklch(0.7_0.18_266)_10%,oklch(0.75_0.17_300)_18%,transparent_28%)_border-box]",
        "[mask:linear-gradient(#000_0_0)_content-box_exclude,linear-gradient(#000_0_0)]",
        className,
      )}
    />
  );
}
