/**
 * Layout de forças do mapa, sem dependência: repulsão entre todos os nós, molas nas arestas e
 * uma gravidade fraca para o centro. A "temperatura" (`alpha`) cai a cada passo; quando fica
 * abaixo de `ALPHA_MIN`, o layout está estável e o desenho para de pedir quadros.
 *
 * O(n²) por passo: com o corte do núcleo (até 500 nós) cabe folgado num quadro.
 */

export interface LayoutNode {
  id: string;
  x: number;
  y: number;
  vx: number;
  vy: number;
  /** Raio desenhado (também usado para não sobrepor nós). */
  r: number;
}

export interface LayoutLink {
  source: number;
  target: number;
  /** Mola mais fraca entre nós muito ligados (um hub não é puxado por todos ao mesmo tempo). */
  strength: number;
  /** Quanto do ajuste cabe ao alvo: o nó com menos ligações é quem mais se move. */
  bias: number;
}

export interface Layout {
  nodes: LayoutNode[];
  links: LayoutLink[];
  alpha: number;
}

export const ALPHA_MIN = 0.005;
const ALPHA_DECAY = 0.0228; // ~300 passos até estabilizar
const REPULSION = 150;
const SPRING = 0.7;
const LINK_DISTANCE = 30;
const GRAVITY = 0.05;
/** Repulsão só até esta distância: grupos soltos não são empurrados para longe. */
const REPULSION_RANGE2 = 250 * 250;
const FRICTION = 0.6;

/** Posição inicial determinística (espiral de Fibonacci): mesmo mapa, mesmo desenho. */
export function createLayout(
  nodes: { id: string; r: number }[],
  edges: { from: string; to: string }[],
): Layout {
  const index = new Map(nodes.map((n, i) => [n.id, i]));
  const golden = Math.PI * (3 - Math.sqrt(5));
  return {
    nodes: nodes.map((n, i) => {
      const radius = 12 * Math.sqrt(i + 0.5);
      return {
        id: n.id,
        r: n.r,
        x: radius * Math.cos(i * golden),
        y: radius * Math.sin(i * golden),
        vx: 0,
        vy: 0,
      };
    }),
    links: withStrength(
      edges.flatMap((e) => {
        const source = index.get(e.from);
        const target = index.get(e.to);
        return source === undefined || target === undefined || source === target
          ? []
          : [{ source, target }];
      }),
      nodes.length,
    ),
    alpha: 1,
  };
}

/** Força e divisão de cada mola pelo grau dos nós (como no d3-force). */
function withStrength(pairs: { source: number; target: number }[], n: number): LayoutLink[] {
  const degree = new Array<number>(n).fill(0);
  for (const { source, target } of pairs) {
    degree[source] = (degree[source] ?? 0) + 1;
    degree[target] = (degree[target] ?? 0) + 1;
  }
  return pairs.map(({ source, target }) => {
    const ds = degree[source] ?? 1;
    const dt = degree[target] ?? 1;
    return { source, target, strength: 1 / Math.min(ds, dt), bias: ds / (ds + dt) };
  });
}

export function isStable(layout: Layout): boolean {
  return layout.alpha < ALPHA_MIN;
}

/** Um passo da simulação (muta o layout). */
export function step(layout: Layout): void {
  const { nodes, links } = layout;
  const alpha = layout.alpha;
  for (let i = 0; i < nodes.length; i++) {
    const a = nodes[i];
    if (!a) continue;
    for (let j = i + 1; j < nodes.length; j++) {
      const b = nodes[j];
      if (!b) continue;
      let dx = b.x - a.x;
      let dy = b.y - a.y;
      let d2 = dx * dx + dy * dy;
      if (d2 < 0.01) {
        // Nós na mesma posição: afasta numa direção fixa (determinística).
        dx = 0.1 * ((i % 7) - 3) || 0.1;
        dy = 0.1 * ((j % 5) - 2) || 0.1;
        d2 = dx * dx + dy * dy;
      }
      if (d2 > REPULSION_RANGE2) continue;
      const d = Math.sqrt(d2);
      const min = a.r + b.r + 6;
      const push = (REPULSION / d + (d < min ? (min - d) * 0.5 : 0)) * alpha;
      const fx = (dx / d) * push;
      const fy = (dy / d) * push;
      a.vx -= fx;
      a.vy -= fy;
      b.vx += fx;
      b.vy += fy;
    }
  }
  for (const link of links) {
    const a = nodes[link.source];
    const b = nodes[link.target];
    if (!a || !b) continue;
    const dx = b.x - a.x;
    const dy = b.y - a.y;
    const d = Math.sqrt(dx * dx + dy * dy) || 0.01;
    const pull = (d - LINK_DISTANCE - a.r - b.r) * SPRING * link.strength * alpha;
    const fx = (dx / d) * pull;
    const fy = (dy / d) * pull;
    a.vx += fx * (1 - link.bias);
    a.vy += fy * (1 - link.bias);
    b.vx -= fx * link.bias;
    b.vy -= fy * link.bias;
  }
  for (const n of nodes) {
    n.vx = (n.vx - n.x * GRAVITY * alpha) * FRICTION;
    n.vy = (n.vy - n.y * GRAVITY * alpha) * FRICTION;
    n.x += n.vx;
    n.y += n.vy;
  }
  layout.alpha *= 1 - ALPHA_DECAY;
}

/** Retângulo que contém todos os nós (para enquadrar o mapa na tela). */
export function bounds(layout: Layout): { minX: number; minY: number; maxX: number; maxY: number } {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const n of layout.nodes) {
    minX = Math.min(minX, n.x - n.r);
    minY = Math.min(minY, n.y - n.r);
    maxX = Math.max(maxX, n.x + n.r);
    maxY = Math.max(maxY, n.y + n.r);
  }
  return Number.isFinite(minX)
    ? { minX, minY, maxX, maxY }
    : { minX: -1, minY: -1, maxX: 1, maxY: 1 };
}

/** Nó sob o ponto (coordenadas do mapa), o de cima primeiro. */
export function hitTest(layout: Layout, x: number, y: number): number | null {
  for (let i = layout.nodes.length - 1; i >= 0; i--) {
    const n = layout.nodes[i];
    if (!n) continue;
    const dx = n.x - x;
    const dy = n.y - y;
    if (dx * dx + dy * dy <= (n.r + 3) * (n.r + 3)) return i;
  }
  return null;
}
