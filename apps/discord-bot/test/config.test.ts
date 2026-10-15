import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { loadConfig, loadEnv } from "../src/config.ts";

const FULL = {
  DISCORD_TOKEN: "token-secreto-de-teste",
  DISCORD_APP_ID: "1",
  DISCORD_GUILD_ID: "2",
  NEXUS_CLI: "nexus",
  NEXUS_DATA_DIR: "dados",
};

describe("variáveis de ambiente", () => {
  it("lists every missing variable", () => {
    expect(() => loadEnv({})).toThrow(/DISCORD_TOKEN.*DISCORD_GUILD_ID/s);
  });

  it("never echoes the token in errors", () => {
    const { NEXUS_DATA_DIR: _omit, ...partial } = FULL;
    let message = "";
    try {
      loadEnv(partial);
    } catch (e) {
      message = (e as Error).message;
    }
    expect(message).toContain("NEXUS_DATA_DIR");
    expect(message).not.toContain("token-secreto-de-teste");
  });

  it("reads a complete environment", () => {
    expect(loadEnv(FULL)).toEqual({
      token: "token-secreto-de-teste",
      appId: "1",
      guildId: "2",
      nexusCli: "nexus",
      nexusDataDir: "dados",
    });
  });
});

describe("bot.config.json", () => {
  const write = (value: unknown) => {
    const path = join(mkdtempSync(join(tmpdir(), "nexus-bot-cfg-")), "bot.config.json");
    writeFileSync(path, JSON.stringify(value));
    return path;
  };

  const base = { publicProject: "nexus", teamProjects: ["otimiza"], teamChannelIds: ["100"] };

  it("reads a valid file; announcements are off without channel IDs", () => {
    expect(loadConfig(write(base))).toEqual({
      ...base,
      announceChannelId: null,
      teamAlertsChannelId: null,
      githubRepo: "EduardoxDev/Nexus",
      moderation: null,
      management: null,
    });
  });

  it("reads the announcement channels", () => {
    const cfg = loadConfig(
      write({
        ...base,
        announceChannelId: "200000",
        teamAlertsChannelId: "100",
        githubRepo: "a/b",
      }),
    );
    expect(cfg.announceChannelId).toBe("200000");
    expect(cfg.teamAlertsChannelId).toBe("100");
    expect(cfg.githubRepo).toBe("a/b");
  });

  it("team alerts only go to a team channel", () => {
    expect(() => loadConfig(write({ ...base, teamAlertsChannelId: "999999" }))).toThrow(
      /teamChannelIds/,
    );
  });

  it("refuses channel IDs that are not numbers", () => {
    expect(() => loadConfig(write({ ...base, announceChannelId: "#anuncios" }))).toThrow(
      /announceChannelId/,
    );
  });

  it("names the invalid field", () => {
    expect(() => loadConfig(write({ publicProject: "nexus", teamProjects: "x" }))).toThrow(
      /teamProjects/,
    );
  });

  it("refuses a project that is both public and team", () => {
    expect(() =>
      loadConfig(write({ publicProject: "nexus", teamProjects: ["NEXUS"], teamChannelIds: [] })),
    ).toThrow(/público e da equipe/);
  });
});

describe("identificação do bot", () => {
  it("the MCP handshake uses the version from package.json", async () => {
    const { CLIENT_INFO } = await import("../src/mcp/client.ts");
    const pkg = JSON.parse(
      readFileSync(join(import.meta.dirname, "..", "package.json"), "utf8"),
    ) as { version: string };
    expect(CLIENT_INFO.version).toBe(pkg.version);
  });
});
