import { changePlan } from "../../billing/subscriptions";
import { requireSession } from "../middleware/requireSession";
import type { Route } from "../app";

export const billingRoutes: Route[] = [
  {
    method: "POST",
    path: "/api/billing/subscription",
    handler: requireSession(async (req, session) => {
      const { plan } = (await req.json()) as { plan: "free" | "pro" | "team" };
      return Response.json(await changePlan(session.userId, plan));
    }),
  },
];
