import { cartTotalCents, type CartItem } from "./cart";

export interface PaymentGateway {
  charge(amountCents: number, token: string): Promise<{ transactionId: string }>;
}

export class PaymentError extends Error {}

/** Cobra o carrinho; recusa valores zerados. */
export async function checkoutCart(
  gateway: PaymentGateway,
  items: CartItem[],
  paymentToken: string,
): Promise<string> {
  const amount = cartTotalCents(items);
  if (amount <= 0) throw new PaymentError("empty_cart");
  const { transactionId } = await gateway.charge(amount, paymentToken);
  return transactionId;
}
