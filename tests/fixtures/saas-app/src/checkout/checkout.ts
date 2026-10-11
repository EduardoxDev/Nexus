import { createCharge } from "../billing/stripeClient";
import { createInvoice } from "../billing/invoices";
import { sendOrderConfirmation } from "../notifications/email";
import { saveOrder } from "../orders/orderRepository";
import { cartSubtotal, type CartItem } from "./cart";
import { shippingCost } from "./shipping";

/** Finaliza o pedido: calcula total, cobra, gera fatura e envia e-mail de confirmação. */
export async function checkout(userId: string, items: CartItem[], zip: string, paymentToken = "pm_card") {
  const subtotal = cartSubtotal(items);
  const total = subtotal + shippingCost(zip, subtotal);
  const chargeId = await createCharge(total, paymentToken);
  const order = await saveOrder(userId, items, total, chargeId);
  await createInvoice(order.id, total);
  await sendOrderConfirmation(userId, order.id, total);
  return order;
}
