import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import type { MapNode, MapRequest, ProjectMap, ProjectSummary } from "@/lib/bindings";
import { renderApp, setupIpc } from "@/test/harness";

const NOW = Date.now();

const project: ProjectSummary = {
  project: {
    id: 1,
    name: "acme",
    rootPath: "C:\\repos\\acme",
    createdAt: NOW,
    lastOpenedAt: NOW,
    lastIndexedAt: NOW,
    indexGeneration: 2,
    headRef: null,
    headCommit: null,
  },
  rootExists: true,
  counts: { total: 10, indexed: 10, skipped: 0, failed: 0, indexedBytes: 1000, indexedLines: 100 },
  activeJob: null,
  lastJob: null,
};

function node(id: string, language: string, inDegree: number, outDegree: number): MapNode {
  return {
    id,
    folder: id.split("/").slice(0, -1).join("/"),
    language,
    files: 1,
    inDegree,
    outDegree,
    centrality: 1,
  };
}

function fileMap(): ProjectMap {
  return {
    nodes: [
      node("src/db/client.ts", "typescript", 3, 0),
      node("src/auth/session.ts", "typescript", 1, 1),
      node("scripts/seed.py", "python", 0, 1),
    ],
    edges: [
      { from: "src/auth/session.ts", to: "src/db/client.ts", weight: 1 },
      { from: "scripts/seed.py", to: "src/db/client.ts", weight: 1 },
    ],
    totalNodes: 40,
    totalEdges: 60,
    grouped: false,
  };
}

function folderMap(): ProjectMap {
  return {
    nodes: [
      { ...node("src/db", "typescript", 1, 0), folder: "src/db", files: 2 },
      { ...node("src/auth", "typescript", 0, 1), folder: "src/auth", files: 4 },
    ],
    edges: [{ from: "src/auth", to: "src/db", weight: 3 }],
    totalNodes: 2,
    totalEdges: 1,
    grouped: true,
  };
}

async function openMap(user: ReturnType<typeof userEvent.setup>) {
  renderApp();
  const nav = await screen.findByRole("navigation", { name: "Navegação principal" });
  await user.click(within(nav).getByRole("button", { name: "Mapa" }));
}

function mapRequests(calls: { cmd: string; args: Record<string, unknown> }[]): MapRequest[] {
  return calls.filter((c) => c.cmd === "get_project_map").map((c) => c.args.request as MapRequest);
}

describe("Mapa", () => {
  it("mostra a tabela dos mais centrais, a legenda e o resumo do corte", async () => {
    const user = userEvent.setup();
    setupIpc({ list_projects: () => [project], get_project_map: () => fileMap() });
    await openMap(user);
    const table = await screen.findByRole("complementary", { name: "Mais centrais (até 30)" });
    const rows = within(table).getAllByRole("row");
    expect(rows).toHaveLength(4);
    const first = rows[1];
    if (!first) throw new Error("sem linha");
    expect(within(first).getByRole("button", { name: "src/db/client.ts" })).toBeInTheDocument();
    expect(within(first).getByText("client.ts")).toBeInTheDocument();
    expect(within(first).getByText("src/db")).toBeInTheDocument();
    expect(within(first).getByText("3")).toBeInTheDocument();
    expect(screen.getByText("3 de 40 arquivos (os mais centrais)")).toBeInTheDocument();
    const legend = screen.getByRole("list", { name: "Cores por linguagem" });
    expect(within(legend).getByText("typescript")).toBeInTheDocument();
    expect(within(legend).getByText("python")).toBeInTheDocument();
    expect(
      screen.getByRole("img", { name: /Mapa do código com 3 nós e 2 ligações/ }),
    ).toBeInTheDocument();
  });

  it("filtra por pasta e agrupa por pasta, pedindo o mapa certo ao núcleo", async () => {
    const user = userEvent.setup();
    const { calls } = setupIpc({
      list_projects: () => [project],
      get_project_map: (args) =>
        (args.request as MapRequest).groupByFolder ? folderMap() : fileMap(),
    });
    await openMap(user);
    await screen.findByRole("complementary", { name: "Mais centrais (até 30)" });
    await user.type(screen.getByRole("searchbox", { name: "Filtrar por pasta" }), "/src/auth/");
    await user.click(screen.getByRole("button", { name: "Filtrar" }));
    await user.click(screen.getByRole("button", { name: "Agrupar por pasta" }));
    expect(await screen.findByText("2 de 2 pastas (os mais centrais)")).toBeInTheDocument();
    expect(mapRequests(calls)).toContainEqual({
      projectId: 1,
      folder: "src/auth",
      groupByFolder: false,
    });
    expect(mapRequests(calls).at(-1)).toEqual({
      projectId: 1,
      folder: "src/auth",
      groupByFolder: true,
    });

    // Clicar numa pasta desagrupa e filtra por ela.
    const table = screen.getByRole("complementary", { name: "Mais centrais (até 30)" });
    await user.click(within(table).getByRole("button", { name: "src/db" }));
    expect(await screen.findByText("3 de 40 arquivos (os mais centrais)")).toBeInTheDocument();
    expect(mapRequests(calls).at(-1)).toEqual({
      projectId: 1,
      folder: "src/db",
      groupByFolder: false,
    });
  });

  it("clicar num arquivo abre a prévia dele na Busca", async () => {
    const user = userEvent.setup();
    const { calls } = setupIpc({
      list_projects: () => [project],
      get_project_map: () => fileMap(),
      get_file_context: (args) => ({
        relPath: args.relPath as string,
        language: "typescript",
        kind: "code",
        startLine: 1,
        endLine: 3,
        totalLines: 3,
        text: `conteúdo de ${args.relPath as string}`,
        freshness: "fresh",
        contentHash: "abc",
        indexedAt: NOW,
      }),
    });
    await openMap(user);
    const table = await screen.findByRole("complementary", { name: "Mais centrais (até 30)" });
    await user.click(within(table).getByRole("button", { name: "src/db/client.ts" }));
    const preview = await screen.findByRole("complementary", { name: "Prévia" });
    expect(await within(preview).findByText("conteúdo de src/db/client.ts")).toBeInTheDocument();
    expect(calls).toContainEqual({
      cmd: "get_file_context",
      args: { projectId: 1, relPath: "src/db/client.ts", startLine: 1, endLine: 400 },
    });
  });

  it("explica quando ainda não há imports entre arquivos", async () => {
    const user = userEvent.setup();
    setupIpc({
      list_projects: () => [project],
      get_project_map: () => ({
        nodes: [],
        edges: [],
        totalNodes: 0,
        totalEdges: 0,
        grouped: false,
      }),
    });
    await openMap(user);
    expect(await screen.findByText("Nenhum import entre arquivos ainda")).toBeInTheDocument();
  });
});
