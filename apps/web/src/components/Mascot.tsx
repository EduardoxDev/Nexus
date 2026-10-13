/**
 * A nuvem da NEXUS como personagem: os olhos acompanham o cursor e piscam de vez em quando.
 * É o único elemento "ousado" da página; com movimento reduzido, fica parada olhando para frente.
 */
import { motion, useReducedMotion, useSpring } from "motion/react";
import { useEffect, useRef, useState } from "react";

const PUPIL_RANGE = 7;

function useEyeTarget(ref: React.RefObject<SVGSVGElement | null>, reduce: boolean) {
  const x = useSpring(0, { stiffness: 120, damping: 14 });
  const y = useSpring(0, { stiffness: 120, damping: 14 });
  useEffect(() => {
    if (reduce) return;
    const onMove = (e: PointerEvent) => {
      const el = ref.current;
      if (!el) return;
      const r = el.getBoundingClientRect();
      const cx = r.left + r.width * 0.6;
      const cy = r.top + r.height * 0.68;
      const dx = e.clientX - cx;
      const dy = e.clientY - cy;
      const dist = Math.hypot(dx, dy) || 1;
      const k = Math.min(1, dist / 260);
      x.set((dx / dist) * PUPIL_RANGE * k);
      y.set((dy / dist) * PUPIL_RANGE * k);
    };
    window.addEventListener("pointermove", onMove, { passive: true });
    return () => window.removeEventListener("pointermove", onMove);
  }, [ref, reduce, x, y]);
  return { x, y };
}

function useBlink(reduce: boolean) {
  const [closed, setClosed] = useState(false);
  useEffect(() => {
    if (reduce) return;
    let timer = 0;
    const loop = () => {
      timer = window.setTimeout(
        () => {
          setClosed(true);
          window.setTimeout(() => setClosed(false), 140);
          loop();
        },
        3200 + Math.random() * 3800,
      );
    };
    loop();
    return () => window.clearTimeout(timer);
  }, [reduce]);
  return closed;
}

function Eye({
  cx,
  cy,
  r,
  pupil,
  closed,
}: {
  cx: number;
  cy: number;
  r: number;
  pupil: { x: ReturnType<typeof useSpring>; y: ReturnType<typeof useSpring> };
  closed: boolean;
}) {
  return (
    <motion.g
      animate={{ scaleY: closed ? 0.08 : 1 }}
      transition={{ duration: 0.07 }}
      style={{ transformOrigin: `${cx}px ${cy}px` }}
    >
      <circle cx={cx} cy={cy} r={r} fill="#fff" />
      <motion.g style={{ x: pupil.x, y: pupil.y }}>
        <circle cx={cx} cy={cy} r={r * 0.72} fill="#1b2b6b" />
        <circle cx={cx} cy={cy} r={r * 0.72} fill="url(#iris)" />
        <path
          d={`M ${cx + r * 0.18} ${cy - r * 0.42} l ${r * 0.08} ${r * 0.2} l ${r * 0.2} ${r * 0.08} l -${r * 0.2} ${r * 0.08} l -${r * 0.08} ${r * 0.2} l -${r * 0.08} -${r * 0.2} l -${r * 0.2} -${r * 0.08} l ${r * 0.2} -${r * 0.08} z`}
          fill="#fff"
        />
        <circle cx={cx + r * 0.42} cy={cy - r * 0.05} r={r * 0.07} fill="#fff" />
      </motion.g>
    </motion.g>
  );
}

export function Mascot({ className }: { className?: string }) {
  const ref = useRef<SVGSVGElement>(null);
  const reduce = useReducedMotion() ?? false;
  const pupil = useEyeTarget(ref, reduce);
  const closed = useBlink(reduce);
  return (
    <svg
      ref={ref}
      viewBox="0 0 320 320"
      className={className}
      role="img"
      aria-label="A nuvem da NEXUS, mascote do app, olhando para você"
    >
      <defs>
        <linearGradient id="fundo" x1="0" y1="0" x2="0.4" y2="1">
          <stop offset="0" stopColor="#3f63ff" />
          <stop offset="1" stopColor="#2348ce" />
        </linearGradient>
        <linearGradient id="nuvem" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#ffffff" />
          <stop offset="1" stopColor="#dde4ff" />
        </linearGradient>
        <radialGradient id="iris" cx="0.4" cy="0.3" r="0.8">
          <stop offset="0" stopColor="#3a52b8" stopOpacity="0.9" />
          <stop offset="1" stopColor="#101c4a" stopOpacity="0" />
        </radialGradient>
        <clipPath id="cartao">
          <rect width="320" height="320" rx="76" />
        </clipPath>
      </defs>
      <g clipPath="url(#cartao)">
        <rect width="320" height="320" fill="url(#fundo)" />
        {/* Ondas mais claras no céu, como no ícone. */}
        <path
          d="M0 96 C 70 70, 120 120, 200 86 S 300 70, 320 92 V 0 H 0 Z"
          fill="#5b7bff"
          opacity="0.35"
        />
        {/* Nuvem: círculos fundidos, saindo pelo canto inferior direito. */}
        <g fill="url(#nuvem)">
          <circle cx="182" cy="196" r="96" />
          <circle cx="110" cy="244" r="70" />
          <circle cx="262" cy="170" r="62" />
          <circle cx="226" cy="122" r="54" />
          <circle cx="300" cy="250" r="80" />
          <rect x="96" y="230" width="230" height="100" />
        </g>
        {/* Sobrancelhas tímidas. */}
        <path
          d="M150 168 q 10 -14 24 -8"
          stroke="#1b2b6b"
          strokeWidth="5"
          strokeLinecap="round"
          fill="none"
          opacity="0.8"
        />
        <path
          d="M236 150 q 14 -8 24 2"
          stroke="#1b2b6b"
          strokeWidth="5"
          strokeLinecap="round"
          fill="none"
          opacity="0.8"
        />
        <Eye cx={180} cy={214} r={30} pupil={pupil} closed={closed} />
        <Eye cx={252} cy={196} r={32} pupil={pupil} closed={closed} />
      </g>
    </svg>
  );
}
