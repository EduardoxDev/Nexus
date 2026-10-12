import { mkdtempSync, writeFileSync } from "node:fs";
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

  it("reads a valid file", () => {
    const cfg = { publicProject: "nexus", teamProjects: ["otimiza"], teamChannelIds: ["100"] };
    expect(loadConfig(write(cfg))).toEqual(cfg);
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
