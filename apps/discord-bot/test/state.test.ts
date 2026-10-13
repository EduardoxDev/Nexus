import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { BotState } from "../src/state.ts";

describe("estado do bot", () => {
  it("remembers values and what was already seen, across restarts", () => {
    const dir = mkdtempSync(join(tmpdir(), "nexus-bot-state-"));
    const file = join(dir, "bot.db");
    const a = new BotState(file);
    a.set("release.announced", "v0.2.0");
    expect(a.unseen("alert:x", ["1", "2"])).toEqual(["1", "2"]);
    a.close();

    const b = new BotState(file);
    expect(b.get("release.announced")).toBe("v0.2.0");
    expect(b.unseen("alert:x", ["1", "2", "3"])).toEqual(["3"]);
    b.close();
    rmSync(dir, { recursive: true, force: true });
  });

  it("forgets resolved items so they are announced again if they come back", () => {
    const s = new BotState(":memory:");
    s.unseen("alert:x", ["1", "2"]);
    s.forgetExcept("alert:x", ["2"]);
    expect(s.unseen("alert:x", ["1", "2"])).toEqual(["1"]);
    s.close();
  });
});
