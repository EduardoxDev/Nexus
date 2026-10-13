import { existsSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { AlreadyRunningError, acquireInstanceLock } from "../src/instance.ts";

const lockPath = () => join(mkdtempSync(join(tmpdir(), "nexus-bot-lock-")), "bot.lock");

describe("instância única", () => {
  it("a second instance is refused while the first runs", () => {
    const path = lockPath();
    const first = acquireInstanceLock(path);
    expect(() => acquireInstanceLock(path)).toThrow(AlreadyRunningError);
    first.release();
  });

  it("the lock is free again after release", () => {
    const path = lockPath();
    acquireInstanceLock(path).release();
    expect(existsSync(path)).toBe(false);
    acquireInstanceLock(path).release();
  });

  it("a lock left by a dead process is taken over", () => {
    const path = lockPath();
    writeFileSync(path, "999999999");
    const lock = acquireInstanceLock(path);
    lock.release();
  });

  it("the refusal names the running process", () => {
    const path = lockPath();
    const first = acquireInstanceLock(path);
    expect(() => acquireInstanceLock(path)).toThrow(String(process.pid));
    first.release();
  });
});
