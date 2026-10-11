import { authRoutes } from "./routes/auth.routes";
import { billingRoutes } from "./routes/billing.routes";
import { orderRoutes } from "./routes/orders.routes";
import { rateLimit } from "./middleware/rateLimit";
import { log } from "../utils/logger";

export interface Route {
  method: "GET" | "POST";
  path: string;
  handler: (req: Request) => Promise<Response>;
}

/** Monta a aplicação HTTP registrando todas as rotas. */
export function createApp(): Route[] {
  const routes = [...authRoutes, ...orderRoutes, ...billingRoutes].map((r) => ({
    ...r,
    handler: rateLimit(r.handler),
  }));
  log("info", "rotas registradas", { count: routes.length });
  return routes;
}
