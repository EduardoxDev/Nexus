import { describe, expect, it } from "vitest";

import { hashPassword, verifyPassword } from "../../src/auth/password";

describe("hash de senha", () => {
  it("verifica a senha correta e recusa a errada", async () => {
    const stored = await hashPassword("correta-123");
    expect(await verifyPassword(stored, "correta-123")).toBe(true);
    expect(await verifyPassword(stored, "errada")).toBe(false);
  });
});
