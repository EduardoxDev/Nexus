import { describe, expect, it } from "vitest";

import type { ChannelRef } from "../src/access.ts";
import {
  autocompleteProjects,
  handleNexus,
  nexusCommand,
  type NexusInteraction,
  type ReplyPayload,
  type Tools,
} from "../src/commands/nexus.ts";
import type { BotConfig } from "../src/config.ts";
import { McpUnavailableError } from "../src/mcp/client.ts";

const cfg: BotConfig = {
  publicProject: "nexus",
  teamProjects: ["otimiza"],
  teamChannelIds: ["100"],
  announceChannelId: null,
  teamAlertsChannelId: null,
  githubRepo: "EduardoxDev/Nexus",
  moderation: null,
};
const COMMON: ChannelRef = { id: "999", parentId: null };
const TEAM: ChannelRef = { id: "100", parentId: null };

const SEARCH = {
  hits: [
    {
      relPath: "docs/mcp.md",
      startLine: 5,
      endLine: 20,
      snippet: "claude mcp add nexus",
      reasons: ["Conteúdo corresponde à consulta"],
      freshness: "fresh",
      language: "markdown",
      score: 0.9,
    },
  ],
  warnings: [],
  project: { id: 1, name: "nexus" },
};

function fakeTools(responses: Record<string, unknown> = {}, fail?: Error) {
  const calls: { name: string; args: Record<string, unknown> }[] = [];
  const tools: Tools = {
    async callTool<T>(name: string, args: Record<string, unknown>): Promise<T> {
      calls.push({ name, args });
      if (fail) throw fail;
      return (responses[name] ?? SEARCH) as T;
    },
  };
  return { tools, calls };
}

function fakeInteraction(channel: ChannelRef, sub: string, opts: Record<string, string> = {}) {
  const sent: { kind: "reply" | "editReply" | "deferReply"; payload: ReplyPayload | undefined }[] =
    [];
  const i: NexusInteraction = {
    channel,
    subcommand: sub,
    option: (name) => opts[name] ?? null,
    deferReply: async () => {
      sent.push({ kind: "deferReply", payload: undefined });
    },
    editReply: async (p) => {
      sent.push({ kind: "editReply", payload: p });
    },
    reply: async (p) => {
      sent.push({ kind: "reply", payload: p });
    },
  };
  // Todo o texto visível: conteúdo e embeds.
  const texts = () =>
    sent
      .map((s) =>
        [
          s.payload?.content ?? "",
          ...(s.payload?.embeds ?? []).flatMap((e) => [
            e.author?.name,
            e.title,
            e.description,
            ...(e.fields ?? []).flatMap((f) => [f.name, f.value]),
          ]),
        ].join("\n"),
      )
      .join("\n");
  return { i, sent, texts };
}

describe("/nexus", () => {
  it("pergunta always uses the public project", async () => {
    const { tools, calls } = fakeTools();
    const { i, texts } = fakeInteraction(COMMON, "pergunta", {
      texto: "como conecto no Claude Code?",
    });
    await handleNexus(i, { tools, cfg });
    expect(calls).toEqual([
      {
        name: "search_project",
        args: {
          project: "nexus",
          query: "como conecto no Claude Code?",
          limit: 3,
          kinds: ["doc"],
        },
      },
    ]);
    expect(texts()).toContain("docs/mcp.md · linhas 5–20");
  });

  it("pergunta falls back to code when no docs match", async () => {
    const calls: Record<string, unknown>[] = [];
    const tools: Tools = {
      async callTool<T>(_name: string, args: Record<string, unknown>): Promise<T> {
        calls.push(args);
        return (args.kinds ? { ...SEARCH, hits: [] } : SEARCH) as T;
      },
    };
    const { i, texts } = fakeInteraction(COMMON, "pergunta", { texto: "shippingCost" });
    await handleNexus(i, { tools, cfg });
    expect(calls).toHaveLength(2);
    expect(calls[1]?.kinds).toBeUndefined();
    expect(texts()).toContain("docs/mcp.md · linhas 5–20");
  });

  it("team project is refused outside team channels", async () => {
    const { tools, calls } = fakeTools();
    const { i, sent, texts } = fakeInteraction(COMMON, "buscar", {
      texto: "pagamento",
      projeto: "otimiza",
    });
    await handleNexus(i, { tools, cfg });
    expect(calls).toHaveLength(0);
    expect(sent).toHaveLength(1);
    expect(sent[0]?.kind).toBe("reply");
    expect(sent[0]?.payload?.ephemeral).toBe(true);
    expect(texts()).toContain("Este projeto não pode ser consultado neste canal.");
    expect(texts()).not.toContain("otimiza");
  });

  it("buscar defaults to the team project in team channels", async () => {
    const { tools, calls } = fakeTools();
    const { i } = fakeInteraction(TEAM, "buscar", { texto: "pagamento" });
    await handleNexus(i, { tools, cfg });
    expect(calls[0]?.args.project).toBe("otimiza");
  });

  it("buscar defaults to the public project in common channels", async () => {
    const { tools, calls } = fakeTools();
    const { i } = fakeInteraction(COMMON, "buscar", { texto: "mcp" });
    await handleNexus(i, { tools, cfg });
    expect(calls[0]?.args.project).toBe("nexus");
  });

  it("pack sends the Markdown as an attachment", async () => {
    const { tools, calls } = fakeTools({
      build_context_pack: {
        packId: "p1",
        summary: "3 fonte(s)",
        estimatedTokens: 900,
        tokenBudget: 8000,
        export: "# Context Pack",
        sources: [{ relPath: "src/a.ts", startLine: 1, endLine: 9 }],
      },
    });
    const { i, sent, texts } = fakeInteraction(TEAM, "pack", { tarefa: "login com google" });
    await handleNexus(i, { tools, cfg });
    expect(calls[0]).toEqual({
      name: "build_context_pack",
      args: { project: "otimiza", task: "login com google", format: "markdown" },
    });
    const last = sent.at(-1)?.payload;
    expect(last?.files?.map((f) => f.name)).toContain("context-pack.md");
    expect(texts()).toContain("src/a.ts:1-9");
  });

  it("status lists only visible projects", async () => {
    const project = (name: string) => ({
      id: name.length,
      name,
      rootPath: `C:\\repos\\${name}`,
      rootExists: true,
      indexedFiles: 10,
      skippedFiles: 0,
      indexGeneration: 1,
      indexing: false,
      lastIndexedAt: null,
      headCommit: null,
    });
    const { tools } = fakeTools({
      list_projects: { projects: [project("nexus"), project("otimiza")] },
    });
    const common = fakeInteraction(COMMON, "status");
    await handleNexus(common.i, { tools, cfg });
    expect(common.texts()).toContain("nexus");
    expect(common.texts()).not.toContain("otimiza");
    const team = fakeInteraction(TEAM, "status");
    await handleNexus(team.i, { tools, cfg });
    expect(team.texts()).toContain("otimiza");
  });

  it("replies never allow mentions", async () => {
    const { tools } = fakeTools();
    const { i, sent } = fakeInteraction(COMMON, "pergunta", { texto: "@everyone" });
    await handleNexus(i, { tools, cfg });
    const replies = sent.filter((s) => s.kind !== "deferReply");
    expect(replies.length).toBeGreaterThan(0);
    for (const s of replies) expect(s.payload?.allowedMentions).toEqual({ parse: [] });
  });

  it("tool failures become friendly messages", async () => {
    const { tools } = fakeTools({}, new McpUnavailableError("caiu"));
    const { i, sent } = fakeInteraction(COMMON, "pergunta", { texto: "x" });
    await handleNexus(i, { tools, cfg });
    expect(sent.at(-1)?.payload?.embeds?.[0]?.description).toBe(
      "A NEXUS não respondeu agora. Tente de novo em instantes.",
    );
  });

  it("sends the configured project spelling to NEXUS (M2)", async () => {
    const { tools, calls } = fakeTools();
    const { i } = fakeInteraction(TEAM, "buscar", { texto: "login", projeto: "OTIMIZA" });
    await handleNexus(i, { tools, cfg });
    expect(calls[0]?.args.project).toBe("otimiza");
  });

  it("the whole command has one deadline, even with several calls (M5)", async () => {
    // Cada chamada sozinha caberia no prazo do cliente; a soma não pode passar do prazo total.
    const tools: Tools = {
      callTool: <T>() => new Promise<T>(() => undefined),
    };
    const { i, sent } = fakeInteraction(COMMON, "pergunta", { texto: "x" });
    const logged: string[] = [];
    const started = Date.now();
    await handleNexus(i, { tools, cfg, deadlineMs: 50, log: (l) => logged.push(l) });
    expect(Date.now() - started).toBeLessThan(2_000);
    expect(sent.at(-1)?.payload?.embeds?.[0]?.description).toBe(
      "A NEXUS não respondeu agora. Tente de novo em instantes.",
    );
    expect(logged[0]).toContain("prazo de 50 ms");
  });

  it("versao shows the version in use and the latest release, in any channel", async () => {
    const tools: Tools = {
      callTool: async <T>() => ({}) as T,
      serverVersion: async () => "0.2.0",
    };
    const { i, texts } = fakeInteraction(COMMON, "versao");
    await handleNexus(i, {
      tools,
      cfg,
      latestRelease: async () => ({
        tag: "v0.2.1",
        name: "NEXUS v0.2.1",
        notes: "",
        url: "https://github.com/EduardoxDev/Nexus/releases/tag/v0.2.1",
        assets: [],
      }),
    });
    expect(texts()).toContain("v0.2.0");
    expect(texts()).toContain("v0.2.1");
    expect(texts()).toContain("mais nova");
  });

  it("versao still answers when GitHub is unreachable", async () => {
    const tools: Tools = { callTool: async <T>() => ({}) as T, serverVersion: async () => "0.2.0" };
    const { i, texts } = fakeInteraction(COMMON, "versao");
    await handleNexus(i, {
      tools,
      cfg,
      latestRelease: () => Promise.reject(new Error("sem rede")),
    });
    expect(texts()).toContain("v0.2.0");
    expect(texts()).toContain("nenhuma ainda");
  });

  it("failures are logged with the reason, without user content", async () => {
    const { tools } = fakeTools(
      {},
      new McpUnavailableError("o servidor MCP da NEXUS foi encerrado"),
    );
    const logged: string[] = [];
    const { i } = fakeInteraction(COMMON, "pergunta", { texto: "pergunta-secreta" });
    await handleNexus(i, { tools, cfg, log: (line) => logged.push(line) });
    expect(logged).toEqual([
      "/nexus pergunta falhou: McpUnavailableError: o servidor MCP da NEXUS foi encerrado",
    ]);
    expect(logged.join()).not.toContain("pergunta-secreta");
  });

  it("autocomplete hides team projects", () => {
    expect(autocompleteProjects("", COMMON, cfg)).toEqual([{ name: "nexus", value: "nexus" }]);
    expect(autocompleteProjects("oti", TEAM, cfg)).toEqual([{ name: "otimiza", value: "otimiza" }]);
  });

  it("defines the slash command with its subcommands", () => {
    const json = nexusCommand.toJSON();
    expect(json.name).toBe("nexus");
    expect(json.options?.map((o) => o.name)).toEqual([
      "pergunta",
      "buscar",
      "pack",
      "status",
      "versao",
    ]);
  });
});
