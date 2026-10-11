import { describe, expect, it, vi } from "vitest";

import { createSession, validateSession } from "../../src/auth/session";

vi.mock("../../src/db/client", () => ({ db: () => ({ query: vi.fn().mockResolvedValue({ rows: [] }) }) }));
vi.mock("../../src/config", () => ({ loadConfig: () => ({ sessionTtlHours: 8 }) }));

describe("sessões", () => {
  it("cria sessão com expiração futura", async () => {
    const session = await createSession("u1");
    expect(session.expiresAt.getTime()).toBeGreaterThan(Date.now());
  });

  it("retorna null para sessão inexistente", async () => {
    expect(await validateSession("nao-existe")).toBeNull();
  });
});
