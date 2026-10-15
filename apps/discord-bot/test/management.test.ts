import { GatewayIntentBits } from "discord.js";
import { describe, expect, it } from "vitest";

import {
  handlePanel,
  panelCommand,
  panelCommandFor,
  ticketCommand,
  type PanelInteraction,
} from "../src/commands/panel.ts";
import { intentsFor } from "../src/discord.ts";
import { DEFAULT_WELCOME, parseManagement, type TicketsConfig } from "../src/management/config.ts";
import {
  rolePanel,
  ROLE_PREFIX,
  toggleRole,
  type PanelMessage,
  type RoleButtonPorts,
  type RoleInfo,
} from "../src/management/roles.ts";
import {
  closeTicket,
  openTicket,
  ticketIntro,
  ticketName,
  type ClosePorts,
  type TicketPorts,
} from "../src/management/tickets.ts";
import { formatWelcome } from "../src/management/welcome.ts";
import { BotState } from "../src/state.ts";

describe("config da gestão", () => {
  it("is off when the block is absent and fills defaults", () => {
    expect(parseManagement("x", undefined)).toBeNull();
    expect(parseManagement("x", { welcome: { channelId: "1" } })).toEqual({
      welcome: { channelId: "1", message: DEFAULT_WELCOME },
      roles: [],
      tickets: null,
    });
  });

  it("names the invalid field", () => {
    expect(() => parseManagement("x", { welcome: { channelId: "abc" } })).toThrow(
      /welcome\.channelId/,
    );
    expect(() => parseManagement("x", { roles: [{ roleId: "1", label: "" }] })).toThrow(
      /roles\[0\]\.label/,
    );
    expect(() =>
      parseManagement("x", {
        roles: [
          { roleId: "1", label: "a" },
          { roleId: "1", label: "b" },
        ],
      }),
    ).toThrow(/duas vezes/);
    expect(() =>
      parseManagement("x", {
        roles: Array.from({ length: 26 }, (_, n) => ({ roleId: `${n + 1}`, label: "a" })),
      }),
    ).toThrow(/até 25/);
    expect(() =>
      parseManagement("x", { roles: [{ roleId: "1", label: "a", emoji: "abc" }] }),
    ).toThrow(/roles\[0\]\.emoji/);
    expect(() =>
      parseManagement("x", { tickets: { channelId: "1", staffRoleIds: ["x"] } }),
    ).toThrow(/staffRoleIds\[0\]/);
  });

  it("asks for the members intent only with the welcome on", () => {
    expect(intentsFor({ automod: false, members: true })).toContain(GatewayIntentBits.GuildMembers);
    expect(intentsFor({ automod: false })).not.toContain(GatewayIntentBits.GuildMembers);
  });
});

describe("boas-vindas", () => {
  const member = {
    id: "10",
    displayName: "Ana *",
    isBot: false,
    guildName: "NEXUS",
    memberCount: 42,
  };

  it("mentions only the newcomer and fills the placeholders", () => {
    const m = formatWelcome(
      { channelId: "1", message: "Oi {usuario}, bem-vindo ao {servidor}!" },
      member,
    );
    expect(m?.mentionUserId).toBe("10");
    expect(m?.embeds[0]?.description).toBe("Oi <@10>, bem-vindo ao NEXUS!");
    expect(m?.embeds[0]?.title).toContain("Ana \\*");
    expect(m?.embeds[0]?.footer?.text).toContain("42");
  });

  it("skips bots", () => {
    expect(formatWelcome({ channelId: "1", message: "x" }, { ...member, isBot: true })).toBeNull();
  });
});

const ok: RoleInfo = { exists: true, dangerous: false, editable: true };
const roles = [
  { roleId: "111", label: "Novidades", emoji: "📣" },
  { roleId: "222", label: "Beta", emoji: null },
];

function rolePorts(over: Partial<RoleButtonPorts> = {}) {
  const changes: string[] = [];
  const ports: RoleButtonPorts = {
    userId: "10",
    memberRoleIds: [],
    info: () => ok,
    add: async (id) => {
      changes.push(`+${id}`);
    },
    remove: async (id) => {
      changes.push(`-${id}`);
    },
    ...over,
  };
  return { changes, ports };
}

describe("cargos por botão", () => {
  it("builds one button per role, in rows of five", () => {
    const many = Array.from({ length: 7 }, (_, n) => ({
      roleId: `${n + 1}`,
      label: `C${n}`,
      emoji: null,
    }));
    const p = rolePanel(many);
    expect(p.components.map((r) => r.components.length)).toEqual([5, 2]);
    expect(p.components[0]?.components[0]).toMatchObject({ custom_id: `${ROLE_PREFIX}1` });
  });

  it("toggles the role", async () => {
    const a = rolePorts();
    expect(await toggleRole(`${ROLE_PREFIX}111`, roles, a.ports)).toMatch(/adicionado/);
    const b = rolePorts({ memberRoleIds: ["111"] });
    expect(await toggleRole(`${ROLE_PREFIX}111`, roles, b.ports)).toMatch(/removido/);
    expect([...a.changes, ...b.changes]).toEqual(["+111", "-111"]);
  });

  it("never touches roles outside the config, staff roles or roles above the bot", async () => {
    for (const [id, info] of [
      ["999", ok],
      ["111", { ...ok, dangerous: true }],
      ["111", { ...ok, editable: false }],
      ["111", { ...ok, exists: false }],
    ] as const) {
      const { changes, ports } = rolePorts({ info: () => info });
      await toggleRole(`${ROLE_PREFIX}${id}`, roles, ports);
      expect(changes).toEqual([]);
    }
  });

  it("concurrent clicks change the role once", async () => {
    let release = () => {};
    const { changes, ports } = rolePorts({
      add: (id) =>
        new Promise<void>((r) => {
          changes.push(`+${id}`);
          release = r;
        }),
    });
    const first = toggleRole(`${ROLE_PREFIX}111`, roles, ports);
    expect(await toggleRole(`${ROLE_PREFIX}111`, roles, ports)).toMatch(/Ainda estou/);
    release();
    expect(await first).toMatch(/adicionado/);
    expect(changes).toEqual(["+111"]);
  });

  it("buttons carry Unicode and server emojis as Discord expects", () => {
    const p = rolePanel([
      { roleId: "1", label: "a", emoji: "📣" },
      { roleId: "2", label: "b", emoji: "<a:festa:123>" },
    ]);
    expect(p.components[0]?.components.map((b) => ("emoji" in b ? b.emoji : null))).toEqual([
      { name: "📣" },
      { name: "festa", id: "123", animated: true },
    ]);
  });

  it("reports a Discord failure without crashing", async () => {
    const { ports } = rolePorts({
      add: async () => {
        throw new Error("Missing Permissions");
      },
    });
    expect(await toggleRole(`${ROLE_PREFIX}111`, roles, ports)).toMatch(/Não consegui/);
  });
});

function panelInteraction(over: Partial<PanelInteraction> = {}) {
  const posted: PanelMessage[] = [];
  const replies: string[] = [];
  const i: PanelInteraction = {
    canManageGuild: true,
    subcommand: "cargos",
    post: async (p) => {
      posted.push(p);
    },
    defer: async () => {},
    reply: async (t) => {
      replies.push(t);
    },
    ...over,
  };
  return { i, posted, replies };
}

describe("/painel", () => {
  const cfg = {
    welcome: null,
    roles,
    tickets: { channelId: "5", staffRoleIds: [], logChannelId: null },
  };

  it("lists only the configured subcommands", () => {
    const names = (roles: boolean, tickets: boolean) =>
      panelCommandFor({ roles, tickets })
        .toJSON()
        .options?.map((o) => o.name);
    expect(names(true, false)).toEqual(["cargos"]);
    expect(names(false, true)).toEqual(["tickets"]);
  });

  it("is hidden from people who cannot manage the server", () => {
    expect(panelCommand.toJSON().default_member_permissions).toBe(String(1n << 5n));
    expect(ticketCommand.toJSON().options?.map((o) => o.name)).toEqual(["fechar"]);
  });

  it("refuses without the permission", async () => {
    const x = panelInteraction({ canManageGuild: false });
    await handlePanel(x.i, cfg, () => ok);
    expect(x.posted).toEqual([]);
  });

  it("refuses to publish a panel with a staff role", async () => {
    const x = panelInteraction();
    await handlePanel(x.i, cfg, (id) => (id === "222" ? { ...ok, dangerous: true } : ok));
    expect(x.posted).toEqual([]);
    expect(x.replies[0]).toMatch(/Beta.*equipe/);
  });

  it("publishes the role and ticket panels", async () => {
    const a = panelInteraction();
    await handlePanel(a.i, cfg, () => ok);
    const b = panelInteraction({ subcommand: "tickets" });
    await handlePanel(b.i, cfg, () => ok);
    expect(a.posted).toHaveLength(1);
    expect(b.posted[0]?.components[0]?.components[0]).toMatchObject({
      custom_id: "nexus:ticket:open",
    });
    const c = panelInteraction({ subcommand: "tickets" });
    await handlePanel(c.i, { ...cfg, tickets: null }, () => ok);
    expect(c.posted).toEqual([]);
  });

  it("defers before posting and reports a failed post", async () => {
    const order: string[] = [];
    const x = panelInteraction({
      defer: async () => {
        order.push("defer");
      },
      post: async () => {
        order.push("post");
        throw new Error("Missing Permissions");
      },
    });
    await handlePanel(x.i, cfg, () => ok);
    expect(order).toEqual(["defer", "post"]);
    expect(x.replies[0]).toMatch(/Não consegui publicar/);
  });
});

const tickets: TicketsConfig = { channelId: "5", staffRoleIds: ["77"], logChannelId: null };

function ticketPorts(over: Partial<TicketPorts> = {}) {
  let n = 0;
  const calls = {
    created: [] as string[],
    added: [] as string[],
    intros: [] as string[],
    deleted: [] as string[],
    logs: 0,
  };
  const ports: TicketPorts = {
    createThread: async (name) => {
      calls.created.push(name);
      return `t${++n}`;
    },
    addMember: async (_t, u) => {
      calls.added.push(u);
    },
    postIntro: async (_t, m) => {
      calls.intros.push(m.content);
    },
    deleteThread: async (t) => {
      calls.deleted.push(t);
    },
    isOpen: async () => true,
    log: async () => {
      calls.logs++;
    },
    ...over,
  };
  return { calls, ports };
}

describe("tickets", () => {
  it("names threads like channels", () => {
    expect(ticketName("João.Silva!!")).toBe("ticket-joao-silva");
    expect(ticketName("💥")).toBe("ticket-membro");
  });

  it("calls the person and the staff, and nobody else", () => {
    expect(ticketIntro(tickets, "10").content).toBe("<@10> <@&77>");
  });

  it("opens one ticket per person", async () => {
    const s = new BotState(":memory:");
    const { calls, ports } = ticketPorts();
    const user = { id: "10", username: "ana" };
    expect(await openTicket(tickets, s, user, ports, 1)).toBe("Ticket aberto: <#t1>.");
    expect(await openTicket(tickets, s, user, ports, 2)).toMatch(/já tem um ticket aberto: <#t1>/);
    expect(calls.created).toEqual(["ticket-ana"]);
    expect(calls.added).toEqual(["10"]);
    expect(calls.logs).toBe(1);
    s.close();
  });

  it("a ticket deleted by hand does not block a new one", async () => {
    const s = new BotState(":memory:");
    const { ports } = ticketPorts({ isOpen: async () => false });
    const user = { id: "10", username: "ana" };
    await openTicket(tickets, s, user, ports, 1);
    expect(await openTicket(tickets, s, user, ports, 2)).toBe("Ticket aberto: <#t2>.");
    expect(s.ticket("t1")?.closedBy).toBe("sistema");
    s.close();
  });

  it("concurrent clicks open a single ticket", async () => {
    const s = new BotState(":memory:");
    const { calls, ports } = ticketPorts();
    const user = { id: "10", username: "ana" };
    const [a, b] = await Promise.all([
      openTicket(tickets, s, user, ports, 1),
      openTicket(tickets, s, user, ports, 1),
    ]);
    expect([a, b].filter((t) => t.startsWith("Ticket aberto"))).toHaveLength(1);
    expect(calls.created).toHaveLength(1);
    s.close();
  });

  it("reports a Discord failure without crashing", async () => {
    const s = new BotState(":memory:");
    const { ports } = ticketPorts({
      createThread: async () => {
        throw new Error("Missing Access");
      },
    });
    expect(await openTicket(tickets, s, { id: "10", username: "ana" }, ports, 1)).toMatch(
      /Não consegui/,
    );
    expect(s.openTicketOf("10")).toBeNull();
    s.close();
  });

  it("a half-made ticket is removed and does not block a retry", async () => {
    const s = new BotState(":memory:");
    let fail = true;
    const { calls, ports } = ticketPorts({
      postIntro: async () => {
        if (fail) throw new Error("Missing Access");
      },
    });
    const user = { id: "10", username: "ana" };
    expect(await openTicket(tickets, s, user, ports, 1)).toMatch(/Não consegui/);
    expect(calls.deleted).toEqual(["t1"]);
    expect(s.openTicketOf("10")).toBeNull();
    fail = false;
    expect(await openTicket(tickets, s, user, ports, 2)).toBe("Ticket aberto: <#t2>.");
    s.close();
  });

  function closePorts(archive: () => Promise<void> = async () => {}) {
    const said = {
      announced: [] as string[],
      refused: [] as string[],
      failed: [] as string[],
      archived: 0,
    };
    const ports: ClosePorts = {
      announce: async (t) => {
        said.announced.push(t);
      },
      refuse: async (t) => {
        said.refused.push(t);
      },
      failed: async (t) => {
        said.failed.push(t);
      },
      archive: async () => {
        await archive();
        said.archived++;
      },
      log: async () => {},
    };
    return { said, ports };
  }

  it("the owner and people who manage threads can close", async () => {
    for (const actor of [
      { id: "10", roleIds: [], canManageThreads: false },
      { id: "40", roleIds: [], canManageThreads: true },
    ]) {
      const s = new BotState(":memory:");
      s.recordTicket("t1", "10", 1);
      const x = closePorts();
      await closeTicket(tickets, s, "t1", actor, x.ports, 2);
      expect(x.said.archived).toBe(1);
      expect(s.ticket("t1")).toMatchObject({ closedAt: 2, closedBy: actor.id });
      s.close();
    }
  });

  it("a failed archive keeps the ticket open so closing can be retried", async () => {
    const s = new BotState(":memory:");
    s.recordTicket("t1", "10", 1);
    const owner = { id: "10", roleIds: [], canManageThreads: false };
    const bad = closePorts(async () => {
      throw new Error("Missing Permissions");
    });
    await closeTicket(tickets, s, "t1", owner, bad.ports, 2);
    expect(bad.said.failed).toHaveLength(1);
    expect(s.ticket("t1")?.closedAt).toBeNull();
    const good = closePorts();
    await closeTicket(tickets, s, "t1", owner, good.ports, 3);
    expect(s.ticket("t1")?.closedAt).toBe(3);
    s.close();
  });

  it("closes for the owner or the staff, not for others", async () => {
    const s = new BotState(":memory:");
    s.recordTicket("t1", "10", 1);
    const stranger = closePorts();
    await closeTicket(
      tickets,
      s,
      "t1",
      { id: "20", roleIds: [], canManageThreads: false },
      stranger.ports,
      2,
    );
    expect(stranger.said.archived).toBe(0);
    expect(s.openTicketOf("10")).toBe("t1");

    const staff = closePorts();
    await closeTicket(
      tickets,
      s,
      "t1",
      { id: "30", roleIds: ["77"], canManageThreads: false },
      staff.ports,
      3,
    );
    expect(staff.said.announced).toEqual(["Ticket fechado por <@30>."]);
    expect(staff.said.archived).toBe(1);
    expect(s.ticket("t1")).toMatchObject({ closedAt: 3, closedBy: "30" });

    const again = closePorts();
    await closeTicket(
      tickets,
      s,
      "t1",
      { id: "10", roleIds: [], canManageThreads: false },
      again.ports,
      4,
    );
    expect(again.said.refused).toHaveLength(1);
    s.close();
  });

  it("refuses outside a ticket", async () => {
    const s = new BotState(":memory:");
    const x = closePorts();
    await closeTicket(
      tickets,
      s,
      "nada",
      { id: "10", roleIds: [], canManageThreads: true },
      x.ports,
      1,
    );
    expect(x.said.refused[0]).toMatch(/não é um ticket/);
    s.close();
  });
});
