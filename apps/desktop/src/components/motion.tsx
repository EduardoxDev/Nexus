/**
 * Animações discretas e reutilizáveis. Todas respeitam "movimento reduzido" (configuração do
 * app ou do sistema) via `MotionConfig`/`useReducedMotion`.
 */
import { motion, useReducedMotion } from "motion/react";
import { useEffect, useRef, useState, type ReactNode } from "react";

import { formatInteger } from "@/lib/format";

const EASE_OUT = [0.22, 1, 0.36, 1] as const;

/** Entrada suave (sobe 8 px e aparece); `index` escalona elementos vizinhos. */
export function Reveal({
  children,
  index = 0,
  className,
}: {
  children: ReactNode;
  index?: number;
  className?: string;
}) {
  return (
    <motion.div
      className={className}
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.28, ease: EASE_OUT, delay: Math.min(index, 8) * 0.045 }}
    >
      {children}
    </motion.div>
  );
}

/** Número que conta até o valor (só na primeira vez e quando muda); sem animação se reduzido. */
export function CountUp({ value, durationMs = 600 }: { value: number; durationMs?: number }) {
  const reduce = useReducedMotion() ?? false;
  const [shown, setShown] = useState(reduce ? value : 0);
  const from = useRef(reduce ? value : 0);

  useEffect(() => {
    if (reduce || from.current === value) {
      from.current = value;
      setShown(value);
      return;
    }
    const start = performance.now();
    const origin = from.current;
    let frame = 0;
    // Um único relógio (o carimbo do rAF pode vir de outra origem de tempo).
    const tick = () => {
      const p = Math.min(1, Math.max(0, (performance.now() - start) / durationMs));
      const eased = 1 - (1 - p) ** 3;
      setShown(Math.round(origin + (value - origin) * eased));
      if (p < 1) frame = requestAnimationFrame(tick);
      else from.current = value;
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [value, durationMs, reduce]);

  return <>{formatInteger(shown)}</>;
}
