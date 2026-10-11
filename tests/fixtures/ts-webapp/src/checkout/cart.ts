export interface CartItem {
  sku: string;
  quantity: number;
  unitPriceCents: number;
}

export function cartTotalCents(items: CartItem[]): number {
  return items.reduce((total, item) => total + item.quantity * item.unitPriceCents, 0);
}

export function addItem(items: CartItem[], item: CartItem): CartItem[] {
  const existing = items.find((i) => i.sku === item.sku);
  if (existing) {
    return items.map((i) =>
      i.sku === item.sku ? { ...i, quantity: i.quantity + item.quantity } : i,
    );
  }
  return [...items, item];
}
