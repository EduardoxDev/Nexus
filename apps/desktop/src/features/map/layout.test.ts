import { describe, expect, it } from "vitest";

import { ALPHA_MIN, bounds, createLayout, hitTest, isStable, step } from "./layout";

function run(nodes: number, edges: [number, number][]) {
  const layout = createLayout(
    Array.from({ length: nodes }, (_, i) => ({ id: `n${i}`, r: 6 })),
    edges.map(([a, b]) => ({ from: `n${a}`, to: `n${b}` })),
  );
  let steps = 0;
  while (!isStable(layout)) {
    step(layout);
    steps++;
  }
  return { layout, steps };
}

describe("layout de forças", () => {
  it("estabiliza em um número limitado de passos, sem posições inválidas", () => {
    const edges: [number, number][] = Array.from({ length: 199 }, (_, i) => [i + 1, i % 13]);
    const { layout, steps } = run(200, edges);
    expect(layout.alpha).toBeLessThan(ALPHA_MIN);
    expect(steps).toBeLessThan(400);
    for (const n of layout.nodes) {
      expect(Number.isFinite(n.x) && Number.isFinite(n.y)).toBe(true);
    }
  });

  it("cada arquivo fica perto do grupo que importa, sem nós sobrepostos", () => {
    // Dois grupos: 1–5 importam o 0; 7–11 importam o 6.
    const edges: [number, number][] = [1, 2, 3, 4, 5]
      .map((i): [number, number] => [i, 0])
      .concat([7, 8, 9, 10, 11].map((i): [number, number] => [i, 6]));
    const { layout } = run(12, edges);
    const at = (i: number) => {
      const n = layout.nodes[i];
      if (!n) throw new Error("sem nó");
      return n;
    };
    const dist = (i: number, j: number) => Math.hypot(at(i).x - at(j).x, at(i).y - at(j).y);
    for (const leaf of [1, 2, 3, 4, 5]) expect(dist(leaf, 0)).toBeLessThan(dist(leaf, 6));
    for (const leaf of [7, 8, 9, 10, 11]) expect(dist(leaf, 6)).toBeLessThan(dist(leaf, 0));
    for (let i = 0; i < 12; i++) {
      for (let j = i + 1; j < 12; j++) expect(dist(i, j)).toBeGreaterThan(at(i).r + at(j).r);
    }
  });

  it("é determinístico: o mesmo mapa gera o mesmo desenho", () => {
    const edges: [number, number][] = [
      [1, 0],
      [2, 0],
      [3, 1],
    ];
    const first = run(30, edges).layout.nodes.map((n) => [n.x, n.y]);
    const second = run(30, edges).layout.nodes.map((n) => [n.x, n.y]);
    expect(second).toEqual(first);
  });

  it("ignora arestas para nós fora do mapa e laços", () => {
    const layout = createLayout(
      [
        { id: "a", r: 5 },
        { id: "b", r: 5 },
      ],
      [
        { from: "a", to: "b" },
        { from: "a", to: "fora" },
        { from: "b", to: "b" },
      ],
    );
    expect(layout.links.map((l) => [l.source, l.target])).toEqual([[0, 1]]);
    expect(layout.links[0]).toMatchObject({ strength: 1, bias: 0.5 });
  });

  it("acha o nó sob o cursor e calcula o enquadramento", () => {
    const { layout } = run(3, []);
    const n = layout.nodes[1];
    if (!n) throw new Error("sem nó");
    expect(hitTest(layout, n.x + 2, n.y - 2)).toBe(1);
    expect(hitTest(layout, n.x + 500, n.y)).toBeNull();
    const b = bounds(layout);
    expect(b.minX).toBeLessThan(b.maxX);
    expect(bounds(createLayout([], [])).maxX).toBe(1);
  });
});
