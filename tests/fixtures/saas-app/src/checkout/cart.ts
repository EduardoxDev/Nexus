import type { Cents } from "../utils/money";

export interface CartItem {
  sku: string;
  quantity: number;
  unitPriceCents: Cents;
}

export function cartSubtotal(items: CartItem[]): Cents {
  return items.reduce((sum, item) => sum + item.quantity * item.unitPriceCents, 0);
}
