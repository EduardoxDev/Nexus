import { describe, expect, it } from "vitest";

import { canQuery, findProject, visibleProjects, type ChannelRef } from "../src/access.ts";
import type { BotConfig } from "../src/config.ts";

const cfg: BotConfig = {
  publicProject: "nexus",
  teamProjects: ["otimiza"],
  teamChannelIds: ["100"],
  announceChannelId: null,
  teamAlertsChannelId: null,
  githubRepo: "EduardoxDev/Nexus",
};
const common: ChannelRef = { id: "999", parentId: null };
const team: ChannelRef = { id: "100", parentId: null };

describe("regra de acesso por canal", () => {
  it("public project anywhere", () => {
    expect(canQuery(cfg, common, "nexus")).toBe(true);
  });

  it("team project only in team channels", () => {
    expect(canQuery(cfg, common, "otimiza")).toBe(false);
    expect(canQuery(cfg, team, "otimiza")).toBe(true);
  });

  it("threads inherit the parent channel", () => {
    expect(canQuery(cfg, { id: "555", parentId: "100" }, "otimiza")).toBe(true);
    expect(canQuery(cfg, { id: "555", parentId: "999" }, "otimiza")).toBe(false);
  });

  it("unknown projects are never allowed", () => {
    expect(canQuery(cfg, team, "outro")).toBe(false);
  });

  it("project names are compared without case", () => {
    expect(canQuery(cfg, team, "Otimiza")).toBe(true);
    expect(canQuery(cfg, common, "NEXUS")).toBe(true);
  });

  it("compares names like NEXUS does: only A–Z ignore case (M2)", () => {
    // "K" (sinal de Kelvin) vira "k" no toLowerCase do JavaScript, mas não na NEXUS.
    expect(canQuery({ ...cfg, teamProjects: ["kanban"] }, team, "Kanban")).toBe(false);
    expect(canQuery(cfg, team, "otımiza")).toBe(false); // ı sem ponto
    expect(canQuery(cfg, team, "OTIMIZA")).toBe(true);
  });

  it("returns the configured spelling, which is what goes to NEXUS (M2)", () => {
    expect(findProject(cfg, team, "  OTIMIZA ")).toBe("otimiza");
    expect(findProject(cfg, common, "OTIMIZA")).toBeNull();
  });

  it("visible list hides team projects outside team channels", () => {
    expect(visibleProjects(cfg, common)).toEqual(["nexus"]);
    expect(visibleProjects(cfg, team)).toEqual(["nexus", "otimiza"]);
  });
});
