import { emit } from "@tauri-apps/api/event";
import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import type { IndexJob, ProjectOverview, ProjectSummary } from "@/lib/bindings";
import { resetIndexing } from "@/stores/indexing";
import { renderApp, setupIpc } from "@/test/harness";

const NOW = Date.now();

function summary(over: Partial<ProjectSummary["project"]> = {}): ProjectSummary {
  return {
    project: {
      id: 1,
      name: "shopfront",
      rootPath: "C:\\repos\\shopfront",
      createdAt: NOW - 3_600_000,
      lastOpenedAt: NOW,
      lastIndexedAt: NOW - 60_000,
      indexGeneration: 3,
      headRef: "refs/heads/main",
      headCommit: "89abcdef0123456789abcdef0123456789abcdef",
      ...over,
    },
    rootExists: true,
    counts: {
      total: 15,
      indexed: 13,
      skipped: 2,
      failed: 0,
      indexedBytes: 20480,
      indexedLines: 420,
    },
    activeJob: null,
    lastJob: null,
  };
}

function overview(s: ProjectSummary): ProjectOverview {
  return {
    summary: s,
    languages: [
      { language: "typescript", files: 7, bytes: 12000, lines: 300 },
      { language: "markdown", files: 2, bytes: 3000, lines: 40 },
    ],
    kinds: [
      { kind: "code", files: 6 },
      { kind: "test", files: 1 },
    ],
    statuses: [{ status: "indexed", files: 13 }],
    topDirectories: [{ name: "src", files: 7 }],
    manifests: ["package.json"],
    ciFiles: [".github/workflows/ci.yml"],
    mostChanged: [],
  };
}

function job(over: Partial<IndexJob> = {}): IndexJob {
  return {
    id: "job-1",
    projectId: 1,
    mode: "incremental",
    trigger: "user",
    status: "running",
    phase: "indexing",
    startedAt: NOW,
    heartbeatAt: NOW,
    finishedAt: null,
    discovered: 10,
    toProcess: 10,
    processed: 3,
    unchanged: 0,
    newContents: 3,
    reusedContents: 0,
    skipped: 0,
    removed: 0,
    renamed: 0,
    failed: 0,
    bytesRead: 2048,
    generation: null,
    errorMessage: null,
    ...over,
  };
}

function projectsIpc(initial: ProjectSummary[]) {
  let projects = initial;
  return setupIpc({
    list_projects: () => projects,
    get_project_overview: (args) => {
      const found = projects.find((p) => p.project.id === args.projectId);
      if (!found) throw new Error("não encontrado");
      return overview(found);
    },
    pick_project_folder: () => "C:\\repos\\shopfront",
    add_project: () => {
      projects = [summary({ lastIndexedAt: null })];
      return projects[0];
    },
    remove_project: () => {
      projects = [];
      return null;
    },
    list_jobs: () => [
      job({
        status: "succeeded",
        phase: "done",
        finishedAt: NOW + 900,
        processed: 13,
        discovered: 15,
      }),
    ],
    list_skipped_files: () => [
      {
        relPath: ".env",
        status: "skipped_sensitive",
        reason: "nome corresponde a padrão de credencial (.env)",
        size: 40,
      },
      {
        relPath: "assets/logo.png",
        status: "skipped_binary",
        reason: "extensão de arquivo binário",
        size: 900,
      },
    ],
    touch_project: () => null,
    start_index: () => null,
  });
}

async function openProjects(user: ReturnType<typeof userEvent.setup>) {
  const nav = await screen.findByRole("navigation", { name: "Navegação principal" });
  await user.click(within(nav).getByRole("button", { name: "Projetos" }));
}

describe("Projetos", () => {
  it("adiciona uma pasta pelo seletor e abre o projeto", async () => {
    resetIndexing();
    const user = userEvent.setup();
    const { calls } = projectsIpc([]);
    renderApp();
    await openProjects(user);
    expect(await screen.findByText("Nenhum projeto autorizado")).toBeInTheDocument();
    const main = screen.getByRole("main");
    await user.click(within(main).getByRole("button", { name: "Adicionar projeto" }));
    expect(await screen.findByRole("heading", { name: "shopfront", level: 2 })).toBeInTheDocument();
    expect(calls).toContainEqual({ cmd: "add_project", args: { path: "C:\\repos\\shopfront" } });
  });

  it("mostra estatísticas reais, linguagens e arquivos ignorados", async () => {
    resetIndexing();
    const user = userEvent.setup();
    projectsIpc([summary()]);
    renderApp();
    await openProjects(user);
    expect(await screen.findByRole("heading", { name: "shopfront", level: 2 })).toBeInTheDocument();
    expect(screen.getByText("main · 89abcdef")).toBeInTheDocument();
    const languages = screen.getByRole("list", { name: "Linguagens" });
    expect(within(languages).getByText("typescript")).toBeInTheDocument();
    expect(within(languages).getByText(/7 arquivos · 300 linhas/)).toBeInTheDocument();
    expect(await screen.findByText(".env")).toBeInTheDocument();
    expect(screen.getByText("Sensível")).toBeInTheDocument();
    expect(screen.getByText("Binário")).toBeInTheDocument();
  });

  it("acompanha o progresso ao vivo pelos eventos do core", async () => {
    resetIndexing();
    const user = userEvent.setup();
    projectsIpc([summary()]);
    renderApp();
    await openProjects(user);
    await screen.findByRole("heading", { name: "shopfront", level: 2 });

    await act(async () => {
      await emit("nexus://queue-changed", { running: 1, paused: false, queued: [] });
      await emit("nexus://job-progress", job());
    });
    const main = screen.getByRole("main");
    await waitFor(() =>
      expect(within(main).getAllByText("Indexando 3 de 10").length).toBeGreaterThan(0),
    );
    const bars = within(main).getAllByRole("progressbar");
    expect(bars.some((b) => b.getAttribute("aria-valuenow") === "30")).toBe(true);
    expect(within(main).getByRole("button", { name: "Pausar" })).toBeInTheDocument();
    expect(screen.getByTestId("status-indexing")).toHaveTextContent("Indexando shopfront 3/10");
    // Enquanto indexa, novas indexações ficam desabilitadas.
    expect(within(main).getByRole("button", { name: "Atualizar índice" })).toBeDisabled();

    await act(async () => {
      await emit(
        "nexus://job-finished",
        job({ status: "succeeded", phase: "done", processed: 10 }),
      );
      await emit("nexus://queue-changed", { running: null, paused: false, queued: [] });
    });
    await waitFor(() => expect(within(main).queryByRole("button", { name: "Pausar" })).toBeNull());
  });

  it("só remove do índice após confirmação explícita", async () => {
    resetIndexing();
    const user = userEvent.setup();
    const { calls } = projectsIpc([summary()]);
    renderApp();
    await openProjects(user);
    await screen.findByRole("heading", { name: "shopfront", level: 2 });
    await user.click(screen.getByRole("button", { name: "Remover do índice" }));
    const dialog = await screen.findByRole("dialog", { name: 'Remover "shopfront" do índice?' });
    expect(within(dialog).getByText(/Nenhum arquivo da pasta será alterado/)).toBeInTheDocument();
    expect(calls.some((c) => c.cmd === "remove_project")).toBe(false);

    await user.click(within(dialog).getByRole("button", { name: "Remover do índice" }));
    await waitFor(() =>
      expect(calls).toContainEqual({ cmd: "remove_project", args: { projectId: 1 } }),
    );
    expect(await screen.findByText("Nenhum projeto autorizado")).toBeInTheDocument();
  });

  it("dispara reindexação completa pelo botão", async () => {
    resetIndexing();
    const user = userEvent.setup();
    const { calls } = projectsIpc([summary()]);
    renderApp();
    await openProjects(user);
    await screen.findByRole("heading", { name: "shopfront", level: 2 });
    await user.click(screen.getByRole("button", { name: "Reindexação completa" }));
    await waitFor(() =>
      expect(calls).toContainEqual({ cmd: "start_index", args: { projectId: 1, mode: "full" } }),
    );
  });
});
