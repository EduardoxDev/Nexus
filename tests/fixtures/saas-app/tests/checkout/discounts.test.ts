import { describe, expect, it } from "vitest";

import { applyCoupon, registerCoupon } from "../../src/checkout/discounts";

describe("cupons", () => {
  it("aplica percentual de desconto", () => {
    registerCoupon({ code: "BEMVINDO10", percentOff: 10, expiresAt: new Date("2099-01-01") });
    expect(applyCoupon(10_000, "bemvindo10")).toBe(9_000);
  });

  it("recusa cupom expirado", () => {
    registerCoupon({ code: "VELHO", percentOff: 50, expiresAt: new Date("2000-01-01") });
    expect(() => applyCoupon(10_000, "VELHO")).toThrow("cupom expirado");
  });
});
