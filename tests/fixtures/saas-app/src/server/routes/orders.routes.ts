import { checkout } from "../../checkout/checkout";
import { listOrders } from "../../orders/orderRepository";
import { requireSession } from "../middleware/requireSession";
import type { Route } from "../app";

export const orderRoutes: Route[] = [
  {
    method: "GET",
    path: "/api/orders",
    handler: requireSession(async (_req, session) => Response.json(await listOrders(session.userId))),
  },
  {
    method: "POST",
    path: "/api/orders/checkout",
    handler: requireSession(async (req, session) => {
      const body = (await req.json()) as { items: { sku: string; quantity: number; unitPriceCents: number }[]; zip: string };
      const order = await checkout(session.userId, body.items, body.zip);
      return Response.json(order, { status: 201 });
    }),
  },
];
