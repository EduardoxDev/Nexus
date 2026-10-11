import { describe, expect, it } from "vitest";

import { checkoutCart, PaymentError } from "./payment";

describe("checkoutCart", () => {
  it("cobra o total do carrinho", async () => {
    const gateway = { charge: async () => ({ transactionId: "tx_1" }) };
    const items = [{ sku: "a", quantity: 2, unitPriceCents: 500 }];
    await expect(checkoutCart(gateway, items, "tok")).resolves.toBe("tx_1");
  });

  it("recusa carrinho vazio", async () => {
    const gateway = { charge: async () => ({ transactionId: "never" }) };
    await expect(checkoutCart(gateway, [], "tok")).rejects.toBeInstanceOf(PaymentError);
  });
});
