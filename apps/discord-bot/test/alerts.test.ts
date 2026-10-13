import { describe, expect, it } from "vitest";

import { collectNotices, type StaleIssue } from "../src/alerts.ts";
import type { Tools } from "../src/commands/nexus.ts";
import { BotState } from "../src/state.ts";

const issue = (id: number): StaleIssue => ({
  id,
  kind: "doc_missing_path",
  severity: "warning",
  title: `Documento cita arquivo que não existe (${id})`,
  reason: "docs/setup.md cita src/old.ts",
  recommendation: "Atualize o documento",
  sources: [{ relPath: "docs/setup.md", line: 12 }],
});

/** NEXUS falsa: os alertas e o último job podem mudar entre passadas. */
function fakeNexus() {
  const world = {
    issues: [issue(1)],
    lastJob: { id: "j1", status: "succeeded", finishedAt: 1_000 },
    rootExists: true,
  };
  const calls: string[] = [];
  const tools: Tools = {
    async callTool<T>(name: string, args: Record<string, unknown>): Promise<T> {
      calls.push(`${name}:${String(args.project)}`);
      if (name === "get_stale_context") return { issues: world.issues } as T;
      return {
        projects: [{ name: args.project, rootExists: world.rootExists, lastJob: world.lastJob }],
      } as T;
    },
  };
  return { tools, world, calls };
}

describe("avisos da equipe", () => {
  it("first run only records what already exists", async () => {
    const { tools } = fakeNexus();
    expect(await collectNotices(tools, ["otimiza"], new BotState(":memory:"))).toEqual([]);
  });

  it("reports each new alert once, and again if it comes back", async () => {
    const { tools, world } = fakeNexus();
    const s = new BotState(":memory:");
    await collectNotices(tools, ["otimiza"], s);

    world.issues = [issue(1), issue(2)];
    const n = await collectNotices(tools, ["otimiza"], s);
    expect(n.map((x) => x.issues.map((i) => i.id))).toEqual([[2]]);
    expect(await collectNotices(tools, ["otimiza"], s)).toEqual([]);

    world.issues = [issue(2)]; // o 1 foi resolvido
    await collectNotices(tools, ["otimiza"], s);
    world.issues = [issue(1), issue(2)]; // e voltou
    expect((await collectNotices(tools, ["otimiza"], s))[0]?.issues.map((i) => i.id)).toEqual([1]);
  });

  it("reports a failed indexing job once", async () => {
    const { tools, world } = fakeNexus();
    const s = new BotState(":memory:");
    await collectNotices(tools, ["otimiza"], s);
    world.lastJob = { id: "j2", status: "failed", finishedAt: 2_000 };
    expect((await collectNotices(tools, ["otimiza"], s))[0]?.failedJob).toEqual({
      id: "j2",
      finishedAt: 2_000,
    });
    expect(await collectNotices(tools, ["otimiza"], s)).toEqual([]);
  });

  it("reports a missing project folder once", async () => {
    const { tools, world } = fakeNexus();
    const s = new BotState(":memory:");
    await collectNotices(tools, ["otimiza"], s);
    world.rootExists = false;
    expect((await collectNotices(tools, ["otimiza"], s))[0]?.rootMissing).toBe(true);
    expect(await collectNotices(tools, ["otimiza"], s)).toEqual([]);
  });

  it("only asks NEXUS about the projects it was given", async () => {
    const { tools, calls } = fakeNexus();
    await collectNotices(tools, ["otimiza"], new BotState(":memory:"));
    expect(calls).toEqual(["get_stale_context:otimiza", "get_index_status:otimiza"]);
  });
});
