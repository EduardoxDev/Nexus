import { describe, expect, it } from "vitest";

import { invoiceNumber } from "../../src/billing/invoices";

describe("faturas", () => {
  it("gera número sequencial com ano", () => {
    expect(invoiceNumber(123, 2026)).toBe("ACME-2026-000123");
  });
});
