import { describe, expect, it, vi } from "vitest";

import { checkout } from "../../src/checkout/checkout";

vi.mock("../../src/billing/stripeClient", () => ({ createCharge: vi.fn().mockResolvedValue("ch_1") }));
vi.mock("../../src/billing/invoices", () => ({ createInvoice: vi.fn() }));
vi.mock("../../src/notifications/email", () => ({ sendOrderConfirmation: vi.fn() }));
vi.mock("../../src/orders/orderRepository", () => ({
  saveOrder: vi.fn().mockResolvedValue({ id: "o1", totalCents: 11_500 }),
}));

describe("checkout", () => {
  it("cobra subtotal + frete e salva o pedido", async () => {
    const order = await checkout("u1", [{ sku: "a", quantity: 1, unitPriceCents: 10_000 }], "01001000");
    expect(order.id).toBe("o1");
  });
});
