import { randomUUID } from "node:crypto";

import { db } from "../db/client";
import type { CartItem } from "../checkout/cart";

export interface Order {
  id: string;
  userId: string;
  totalCents: number;
  chargeId: string;
}

export async function saveOrder(userId: string, items: CartItem[], totalCents: number, chargeId: string): Promise<Order> {
  const order = { id: randomUUID(), userId, totalCents, chargeId };
  await db().query("INSERT INTO orders (id, user_id, total_cents, status) VALUES ($1, $2, $3, 'paid')", [
    order.id,
    userId,
    totalCents,
  ]);
  for (const item of items) {
    await db().query("INSERT INTO order_items VALUES ($1, $2, $3, $4)", [order.id, item.sku, item.quantity, item.unitPriceCents]);
  }
  return order;
}

export async function listOrders(userId: string): Promise<Order[]> {
  const { rows } = await db().query("SELECT * FROM orders WHERE user_id = $1 ORDER BY created_at DESC", [userId]);
  return rows;
}
