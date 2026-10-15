import { describe, expect, it } from "vitest";

import { handleMod, modCommand, type ModInteraction, type ModTarget } from "../src/commands/mod.ts";
import type { ReplyPayload } from "../src/commands/nexus.ts";
import type { BotMessage } from "../src/format.ts";
import { applyAutomod, formatModLog, type AutomodPorts } from "../src/moderation/actions.ts";
import {
  evaluate,
  FloodTracker,
  inviteCodes,
  type MessageInfo,
} from "../src/moderation/automod.ts";
import {
  MODERATION_DEFAULTS,
  parseModeration,
  type ModerationConfig,
} from "../src/moderation/config.ts";
import { BotState } from "../src/state.ts";

const cfg = (over: Partial<ModerationConfig> = {}): ModerationConfig => ({
  ...MODERATION_DEFAULTS,
  logChannelId: null,
  modRoleIds: [],
  exemptChannelIds: [],
  exemptRoleIds: [],
  allowedInviteCodes: [],
  ...over,
});

const msg = (over: Partial<MessageInfo> = {}): MessageInfo => ({
  authorId: "100",
  authorIsBot: false,
  roleIds: [],
  channelId: "200",
  parentId: null,
  content: "oi",
  mentionCount: 0,
  at: 1_000_000,
  ...over,
});

describe("config da moderação", () => {
  it("is off when the block is absent", () => {
    expect(parseModeration("x", undefined)).toBeNull();
  });

  it("fills defaults", () => {
    expect(parseModeration("x", {})).toEqual(cfg());
  });

  it("names the invalid field", () => {
    expect(() => parseModeration("x", { maxMentions: 1 })).toThrow(/moderation\.maxMentions/);
    expect(() => parseModeration("x", { modRoleIds: ["abc"] })).toThrow(/moderation\.modRoleIds/);
    expect(() => parseModeration("x", { logChannelId: "canal" })).toThrow(/logChannelId/);
    expect(() => parseModeration("x", { automod: "sim" })).toThrow(/moderation\.automod/);
    expect(() => parseModeration("x", { flood: { seconds: 0 } })).toThrow(/flood\.seconds/);
    expect(() => parseModeration("x", [])).toThrow(/objeto/);
  });
});

describe("regras do automod", () => {
  it("finds invite codes in the usual forms", () => {
    expect(
      inviteCodes(
        "entra aí discord.gg/abc e https://discord.com/invite/xyz-1 ou discordapp.com/invite/Q2",
      ),
    ).toEqual(["abc", "xyz-1", "Q2"]);
    expect(inviteCodes("https://discord.com/channels/1/2")).toEqual([]);
  });

  it("blocks foreign invites but allows the server's own", () => {
    const c = cfg({ allowedInviteCodes: ["Nexus"] });
    const f = new FloodTracker();
    expect(evaluate(c, msg({ content: "discord.gg/nexus" }), f)).toBeNull();
    expect(evaluate(c, msg({ content: "discord.gg/outro" }), f)?.rule).toBe("invite");
    expect(
      evaluate(cfg({ blockInvites: false }), msg({ content: "discord.gg/outro" }), f),
    ).toBeNull();
  });

  it("catches mention spam", () => {
    const f = new FloodTracker();
    expect(evaluate(cfg(), msg({ mentionCount: 5 }), f)).toBeNull();
    expect(evaluate(cfg(), msg({ mentionCount: 6 }), f)?.rule).toBe("mentions");
  });

  it("catches flood once per burst, inside the window only", () => {
    const c = cfg({ flood: { messages: 3, seconds: 5 } });
    const f = new FloodTracker();
    const at = (s: number) => msg({ at: s * 1000 });
    expect(evaluate(c, at(0), f)).toBeNull();
    expect(evaluate(c, at(1), f)).toBeNull();
    expect(evaluate(c, at(2), f)).toBeNull();
    expect(evaluate(c, at(3), f)?.rule).toBe("flood");
    // Zerado depois de agir: a mesma rajada não gera outra advertência na hora.
    expect(evaluate(c, at(3.5), f)).toBeNull();
    // Espaçadas, nunca passam do limite.
    const g = new FloodTracker();
    for (let s = 0; s < 40; s += 2) expect(evaluate(c, at(s), g)).toBeNull();
  });

  it("ignores bots, exempt channels (and their threads) and exempt roles", () => {
    const c = cfg({ exemptChannelIds: ["300"], exemptRoleIds: ["9"] });
    const f = new FloodTracker();
    const bad = { content: "discord.gg/outro" };
    expect(evaluate(c, msg({ ...bad, authorIsBot: true }), f)).toBeNull();
    expect(evaluate(c, msg({ ...bad, channelId: "300" }), f)).toBeNull();
    expect(evaluate(c, msg({ ...bad, channelId: "301", parentId: "300" }), f)).toBeNull();
    expect(evaluate(c, msg({ ...bad, roleIds: ["9"] }), f)).toBeNull();
    expect(evaluate(cfg({ automod: false }), msg(bad), f)).toBeNull();
  });
});

describe("histórico de moderação", () => {
  it("records actions and counts warnings in a window", () => {
    const s = new BotState(":memory:");
    const base = { by: "automod", reason: "r", rule: null, minutes: null } as const;
    s.recordAction({ ...base, userId: "1", action: "warn", at: 10 });
    s.recordAction({ ...base, userId: "1", action: "warn", at: 20 });
    s.recordAction({ ...base, userId: "1", action: "timeout", at: 30, minutes: 10 });
    s.recordAction({ ...base, userId: "2", action: "warn", at: 25 });
    expect(s.warningsSince("1", 15)).toBe(1);
    expect(s.warningsSince("1", 0)).toBe(2);
    expect(s.history("1").map((a) => a.action)).toEqual(["timeout", "warn", "warn"]);
    expect(s.history("1", 1)).toHaveLength(1);
    s.close();
  });
});

function fakePorts(over: Partial<AutomodPorts> = {}) {
  const calls = {
    deleted: 0,
    notes: [] as string[],
    timeouts: [] as number[],
    logs: [] as BotMessage[],
  };
  const ports: AutomodPorts = {
    deleteMessage: async () => {
      calls.deleted++;
    },
    notify: async (_id, text) => {
      calls.notes.push(text);
    },
    timeout: async (_id, ms) => {
      calls.timeouts.push(ms);
    },
    log: async (m) => {
      calls.logs.push(m);
    },
    ...over,
  };
  return { calls, ports };
}

describe("ação do automod", () => {
  const verdict = { rule: "invite", reason: "convite para outro servidor" } as const;

  it("warns, and times out once the threshold is reached", async () => {
    const s = new BotState(":memory:");
    const c = cfg({ warnThreshold: 2, timeoutMinutes: 5 });
    const { calls, ports } = fakePorts();
    expect(await applyAutomod(c, s, msg({ at: 1 }), verdict, ports)).toBe("warned");
    expect(calls.notes[0]).toContain("(1/2)");
    expect(await applyAutomod(c, s, msg({ at: 2 }), verdict, ports)).toBe("timed_out");
    expect(calls.deleted).toBe(2);
    expect(calls.timeouts).toEqual([5 * 60_000]);
    expect(calls.logs).toHaveLength(3);
    expect(s.history("100").map((a) => a.action)).toEqual(["timeout", "warn", "warn"]);
    s.close();
  });

  it("old warnings outside the window do not count", async () => {
    const s = new BotState(":memory:");
    const c = cfg({ warnThreshold: 2, warnWindowHours: 1 });
    const { ports } = fakePorts();
    await applyAutomod(c, s, msg({ at: 0 }), verdict, ports);
    expect(await applyAutomod(c, s, msg({ at: 2 * 3_600_000 }), verdict, ports)).toBe("warned");
    s.close();
  });

  it("keeps the warning when deleting or timing out fails", async () => {
    const s = new BotState(":memory:");
    const { calls, ports } = fakePorts({
      deleteMessage: async () => {
        throw new Error("sem permissão");
      },
      timeout: async () => {
        throw new Error("cargo acima");
      },
    });
    expect(await applyAutomod(cfg({ warnThreshold: 1 }), s, msg(), verdict, ports)).toBe("warned");
    expect(s.history("100").map((a) => a.action)).toEqual(["warn"]);
    expect(calls.notes).toHaveLength(1);
    s.close();
  });

  it("the log entry never pings and does not copy the message", () => {
    const log = formatModLog({
      userId: "1",
      by: "automod",
      action: "warn",
      reason: "convite",
      rule: "invite",
      minutes: null,
      at: 0,
    });
    expect(JSON.stringify(log)).not.toContain("discord.gg");
    expect(log.files).toEqual([]);
  });
});

const target = (over: Partial<ModTarget> = {}): ModTarget => ({
  id: "500",
  isBot: false,
  inGuild: true,
  moderatable: true,
  bannable: true,
  ...over,
});

function interaction(
  over: Partial<ModInteraction> & { args?: Record<string, string | number> } = {},
) {
  const replies: ReplyPayload[] = [];
  const args = over.args ?? {};
  const i: ModInteraction = {
    actorId: "400",
    actorRoleIds: [],
    canModerate: true,
    canBan: true,
    subcommand: "advertir",
    target: target(),
    string: (n) => (typeof args[n] === "string" ? (args[n] as string) : null),
    integer: (n) => (typeof args[n] === "number" ? (args[n] as number) : null),
    reply: async (p) => {
      replies.push(p);
    },
    ...over,
  };
  return { i, replies };
}

function deps(c = cfg()) {
  const state = new BotState(":memory:");
  const done = { timeouts: [] as [string, number][], bans: [] as [string, number][], logs: 0 };
  return {
    done,
    state,
    d: {
      cfg: c,
      state,
      timeout: async (id: string, ms: number) => {
        done.timeouts.push([id, ms]);
      },
      ban: async (id: string, _r: string, secs: number) => {
        done.bans.push([id, secs]);
      },
      log: async () => {
        done.logs++;
      },
      now: () => 5_000,
    },
  };
}

describe("/mod", () => {
  it("is hidden from people without the moderate permission", () => {
    const json = modCommand.toJSON();
    expect(json.default_member_permissions).toBe(String(1n << 40n));
    expect(json.options?.map((o) => o.name)).toEqual([
      "advertir",
      "silenciar",
      "banir",
      "historico",
    ]);
  });

  it("refuses without the configured roles or the permission", async () => {
    const a = interaction({ actorRoleIds: ["1"] });
    const x = deps(cfg({ modRoleIds: ["2"] }));
    await handleMod(a.i, x.d);
    expect(a.replies[0]?.content).toMatch(/equipe de moderação/);

    const b = interaction({ canModerate: false });
    await handleMod(b.i, deps().d);
    expect(b.replies[0]?.content).toMatch(/permissão/);
    expect(x.state.history("500")).toEqual([]);
  });

  it("refuses self, bots and missing targets", async () => {
    for (const over of [
      { target: target({ id: "400" }) },
      { target: target({ isBot: true }) },
      { target: null },
    ]) {
      const { i, replies } = interaction(over);
      const x = deps();
      await handleMod(i, x.d);
      expect(replies[0]?.ephemeral).toBe(true);
      expect(x.state.history("500")).toEqual([]);
    }
  });

  it("warns and records who did it", async () => {
    const { i, replies } = interaction({ args: { motivo: " spam " } });
    const x = deps();
    await handleMod(i, x.d);
    const [a] = x.state.history("500");
    expect(a).toMatchObject({ by: "400", action: "warn", reason: "spam", at: 5_000 });
    expect(x.done.logs).toBe(1);
    expect(replies[0]?.allowedMentions).toEqual({ parse: [] });
  });

  it("times out only people the bot can moderate", async () => {
    const ok = interaction({ subcommand: "silenciar", args: { minutos: 15, motivo: "x" } });
    const x = deps();
    await handleMod(ok.i, x.d);
    expect(x.done.timeouts).toEqual([["500", 15 * 60_000]]);

    const high = interaction({
      subcommand: "silenciar",
      target: target({ moderatable: false }),
      args: { minutos: 15, motivo: "x" },
    });
    const y = deps();
    await handleMod(high.i, y.d);
    expect(y.done.timeouts).toEqual([]);
    expect(high.replies[0]?.content).toMatch(/Não consigo silenciar/);
  });

  it("bans need the ban permission; people outside the server can be banned", async () => {
    const no = interaction({ subcommand: "banir", canBan: false, args: { motivo: "x" } });
    const x = deps();
    await handleMod(no.i, x.d);
    expect(x.done.bans).toEqual([]);

    const away = interaction({
      subcommand: "banir",
      target: target({ inGuild: false, bannable: false }),
      args: { motivo: "x", apagar_dias: 2 },
    });
    const y = deps();
    await handleMod(away.i, y.d);
    expect(y.done.bans).toEqual([["500", 2 * 86_400]]);
    expect(y.state.history("500")[0]?.action).toBe("ban");
  });

  it("shows the history privately", async () => {
    const { i, replies } = interaction({ subcommand: "historico" });
    const x = deps();
    x.state.recordAction({
      userId: "500",
      by: "automod",
      action: "warn",
      reason: "convite",
      rule: "invite",
      minutes: null,
      at: 1_000,
    });
    await handleMod(i, x.d);
    expect(replies[0]?.ephemeral).toBe(true);
    expect(JSON.stringify(replies[0]?.embeds)).toContain("convite");
  });
});
