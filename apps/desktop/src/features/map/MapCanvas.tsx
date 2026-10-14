import { useEffect, useMemo, useRef, useState, type PointerEvent, type WheelEvent } from "react";

import type { MapNode, ProjectMap } from "@/lib/bindings";
import { t } from "@/lib/i18n/pt-BR";
import { bounds, createLayout, hitTest, isStable, step, type Layout } from "./layout";

const s = t.map;

/** Paleta categórica das linguagens (legível no claro e no escuro); o resto fica cinza. */
export const LANGUAGE_COLORS: Record<string, string> = {
  typescript: "oklch(0.66 0.15 255)",
  javascript: "oklch(0.8 0.14 90)",
  python: "oklch(0.68 0.13 165)",
  rust: "oklch(0.68 0.15 45)",
  go: "oklch(0.72 0.11 210)",
  java: "oklch(0.64 0.16 20)",
  csharp: "oklch(0.62 0.15 300)",
};
const OTHER_COLOR = "oklch(0.62 0.01 286)";

export function languageColor(language: string | null): string {
  return (language && LANGUAGE_COLORS[language]) ?? OTHER_COLOR;
}

/** Raio pelo grau de entrada (área cresce com quem importa o arquivo). */
function radius(node: MapNode, maxIn: number): number {
  return 4 + 10 * Math.sqrt(node.inDegree / Math.max(1, maxIn));
}

/** Rótulos visíveis sem hover: só os mais centrais, para não poluir. */
const LABELED = 12;
const STEPS_PER_FRAME = 3;
/** Espera antes de abrir um nó clicado, para reconhecer o duplo clique. */
const DOUBLE_CLICK_MS = 250;

interface View {
  k: number;
  tx: number;
  ty: number;
}

function css(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || "gray";
}

function shortName(id: string): string {
  return id.split("/").at(-1) ?? id;
}

/**
 * Desenho do mapa em canvas: forças animadas até estabilizar (parado com movimento reduzido),
 * zoom na roda, arraste para mover, hover destaca vizinhos e clique abre o nó.
 */
export function MapCanvas({
  map,
  reducedMotion,
  onOpen,
  onHover,
}: {
  map: ProjectMap;
  reducedMotion: boolean;
  onOpen: (node: MapNode) => void;
  onHover: (node: MapNode | null) => void;
}) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const layoutRef = useRef<Layout | null>(null);
  const viewRef = useRef<View>({ k: 1, tx: 0, ty: 0 });
  /** Depois que a pessoa mexe no zoom ou arrasta, o enquadramento automático para. */
  const userMovedRef = useRef(false);
  const hoverRef = useRef<number | null>(null);
  const dragRef = useRef<{ x: number; y: number; moved: boolean } | null>(null);
  const frameRef = useRef<number | null>(null);
  const openTimerRef = useRef<number | null>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });
  /** Tamanho atual para o laço de animação (que roda com o fechamento de um render antigo). */
  const sizeRef = useRef(size);

  const maxIn = useMemo(() => Math.max(1, ...map.nodes.map((n) => n.inDegree)), [map]);
  const neighbors = useMemo(() => {
    const index = new Map(map.nodes.map((n, i) => [n.id, i]));
    const list = map.nodes.map(() => new Set<number>());
    for (const e of map.edges) {
      const a = index.get(e.from);
      const b = index.get(e.to);
      if (a === undefined || b === undefined) continue;
      list[a]?.add(b);
      list[b]?.add(a);
    }
    return list;
  }, [map]);

  // Clique pendente não abre nada depois que a tela some.
  useEffect(
    () => () => {
      if (openTimerRef.current !== null) window.clearTimeout(openTimerRef.current);
    },
    [],
  );

  // Tamanho do contêiner (o canvas acompanha, com a densidade de pixels da tela).
  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const update = () => {
      sizeRef.current = { w: el.clientWidth, h: el.clientHeight };
      setSize(sizeRef.current);
    };
    update();
    const ro = new ResizeObserver(update);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const draw = () => {
    const canvas = canvasRef.current;
    const layout = layoutRef.current;
    const ctx = canvas?.getContext("2d");
    const size = sizeRef.current;
    if (!canvas || !layout || !ctx || size.w === 0) return;
    const dpr = window.devicePixelRatio || 1;
    if (!userMovedRef.current) fit(layout);
    const { k, tx, ty } = viewRef.current;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, size.w, size.h);
    ctx.setTransform(dpr * k, 0, 0, dpr * k, dpr * tx, dpr * ty);

    const hover = hoverRef.current;
    const near = hover === null ? null : neighbors[hover];
    const edge = css("--border-strong");
    const accent = css("--accent");
    ctx.lineWidth = 1 / k;
    for (const link of layout.links) {
      const a = layout.nodes[link.source];
      const b = layout.nodes[link.target];
      if (!a || !b) continue;
      const lit = hover !== null && (link.source === hover || link.target === hover);
      ctx.strokeStyle = lit ? accent : edge;
      ctx.globalAlpha = hover === null ? 0.55 : lit ? 0.95 : 0.12;
      ctx.lineWidth = (lit ? 1.6 : 1) / k;
      ctx.beginPath();
      ctx.moveTo(a.x, a.y);
      ctx.lineTo(b.x, b.y);
      ctx.stroke();
    }
    layout.nodes.forEach((n, i) => {
      const node = map.nodes[i];
      if (!node) return;
      const dim = hover !== null && i !== hover && !near?.has(i);
      ctx.globalAlpha = dim ? 0.2 : 1;
      ctx.fillStyle = languageColor(node.language);
      ctx.beginPath();
      ctx.arc(n.x, n.y, n.r, 0, Math.PI * 2);
      ctx.fill();
      if (i === hover) {
        ctx.strokeStyle = accent;
        ctx.lineWidth = 2.5 / k;
        ctx.stroke();
      }
    });
    ctx.globalAlpha = 1;
    ctx.fillStyle = css("--fg-muted");
    ctx.font = `${11 / k}px Inter Variable, Inter, system-ui, sans-serif`;
    ctx.textAlign = "center";
    layout.nodes.forEach((n, i) => {
      const node = map.nodes[i];
      if (!node) return;
      const show = hover === null ? i < LABELED : i === hover || near?.has(i);
      if (!show) return;
      ctx.fillText(map.grouped ? node.id : shortName(node.id), n.x, n.y + n.r + 12 / k);
    });
  };

  const fit = (layout: Layout) => {
    const size = sizeRef.current;
    const b = bounds(layout);
    const pad = 32;
    const w = Math.max(1, b.maxX - b.minX);
    const h = Math.max(1, b.maxY - b.minY);
    const k = Math.min((size.w - pad * 2) / w, (size.h - pad * 2) / h, 2.5);
    viewRef.current = {
      k,
      tx: size.w / 2 - ((b.minX + b.maxX) / 2) * k,
      ty: size.h / 2 - ((b.minY + b.maxY) / 2) * k,
    };
  };

  const schedule = () => {
    if (frameRef.current !== null) return;
    frameRef.current = requestAnimationFrame(tick);
  };

  const tick = () => {
    frameRef.current = null;
    const layout = layoutRef.current;
    if (!layout) return;
    for (let i = 0; i < STEPS_PER_FRAME && !isStable(layout); i++) step(layout);
    draw();
    if (!isStable(layout)) schedule();
  };

  // Mapa novo: layout novo. Com movimento reduzido, calcula tudo antes e desenha uma vez.
  useEffect(() => {
    const layout = createLayout(
      map.nodes.map((n) => ({ id: n.id, r: radius(n, maxIn) })),
      map.edges,
    );
    layoutRef.current = layout;
    userMovedRef.current = false;
    hoverRef.current = null;
    if (reducedMotion) {
      while (!isStable(layout)) step(layout);
    }
    schedule();
    return () => {
      if (frameRef.current !== null) cancelAnimationFrame(frameRef.current);
      frameRef.current = null;
    };
    // `schedule` lê refs; o layout só recomeça quando o mapa muda.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [map, maxIn, reducedMotion]);

  // Redesenha quando o tamanho muda (o canvas é limpo ao trocar de tamanho).
  useEffect(() => {
    schedule();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [size]);

  const toMap = (clientX: number, clientY: number) => {
    const rect = canvasRef.current?.getBoundingClientRect();
    const { k, tx, ty } = viewRef.current;
    const x = clientX - (rect?.left ?? 0);
    const y = clientY - (rect?.top ?? 0);
    return { x: (x - tx) / k, y: (y - ty) / k, sx: x, sy: y };
  };

  const onWheel = (e: WheelEvent<HTMLCanvasElement>) => {
    const { sx, sy } = toMap(e.clientX, e.clientY);
    const v = viewRef.current;
    const k = Math.min(8, Math.max(0.1, v.k * Math.exp(-e.deltaY * 0.0015)));
    // Zoom no ponto do cursor.
    viewRef.current = { k, tx: sx - ((sx - v.tx) * k) / v.k, ty: sy - ((sy - v.ty) * k) / v.k };
    userMovedRef.current = true;
    schedule();
  };

  const onPointerDown = (e: PointerEvent<HTMLCanvasElement>) => {
    // Só o botão principal move o mapa ou abre nós.
    if (e.button !== 0) return;
    dragRef.current = { x: e.clientX, y: e.clientY, moved: false };
    // Arrastar para fora do mapa continua movendo (jsdom não tem captura: ignora).
    if (typeof e.currentTarget.setPointerCapture === "function") {
      e.currentTarget.setPointerCapture(e.pointerId);
    }
  };

  const onPointerMove = (e: PointerEvent<HTMLCanvasElement>) => {
    const drag = dragRef.current;
    if (drag) {
      const dx = e.clientX - drag.x;
      const dy = e.clientY - drag.y;
      if (drag.moved || Math.abs(dx) + Math.abs(dy) > 3) {
        drag.moved = true;
        userMovedRef.current = true;
        viewRef.current = {
          ...viewRef.current,
          tx: viewRef.current.tx + dx,
          ty: viewRef.current.ty + dy,
        };
        drag.x = e.clientX;
        drag.y = e.clientY;
        schedule();
        return;
      }
    }
    const layout = layoutRef.current;
    if (!layout) return;
    const { x, y } = toMap(e.clientX, e.clientY);
    const hit = hitTest(layout, x, y);
    if (hit !== hoverRef.current) {
      hoverRef.current = hit;
      onHover(hit === null ? null : (map.nodes[hit] ?? null));
      schedule();
    }
  };

  const onPointerUp = (e: PointerEvent<HTMLCanvasElement>) => {
    if (e.button !== 0) return;
    const drag = dragRef.current;
    dragRef.current = null;
    if (!drag || drag.moved) return;
    const layout = layoutRef.current;
    if (!layout) return;
    const { x, y } = toMap(e.clientX, e.clientY);
    const hit = hitTest(layout, x, y);
    const node = hit === null ? undefined : map.nodes[hit];
    if (!node) return;
    // Espera um pouco: se vier um segundo clique (duplo clique = enquadrar), não abre.
    if (openTimerRef.current !== null) window.clearTimeout(openTimerRef.current);
    openTimerRef.current = window.setTimeout(() => {
      openTimerRef.current = null;
      onOpen(node);
    }, DOUBLE_CLICK_MS);
  };

  const onPointerLeave = () => {
    dragRef.current = null;
    if (hoverRef.current !== null) {
      hoverRef.current = null;
      onHover(null);
      schedule();
    }
  };

  const dpr = typeof window === "undefined" ? 1 : window.devicePixelRatio || 1;
  return (
    <div ref={wrapRef} className="relative h-full w-full">
      <canvas
        ref={canvasRef}
        role="img"
        aria-label={s.canvasLabel(map.nodes.length, map.edges.length)}
        width={Math.round(size.w * dpr)}
        height={Math.round(size.h * dpr)}
        style={{ width: size.w, height: size.h }}
        className="block cursor-grab touch-none active:cursor-grabbing"
        onWheel={onWheel}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerLeave={onPointerLeave}
        onDoubleClick={() => {
          if (openTimerRef.current !== null) window.clearTimeout(openTimerRef.current);
          openTimerRef.current = null;
          userMovedRef.current = false;
          schedule();
        }}
      />
    </div>
  );
}
