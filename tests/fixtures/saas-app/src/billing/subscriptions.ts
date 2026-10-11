import { db } from "../db/client";
import { DomainError } from "../utils/errors";

export type Plan = "free" | "pro" | "team";

const PLAN_PRICES_CENTS: Record<Plan, number> = { free: 0, pro: 4900, team: 14900 };

/** Troca o plano (upgrade ou downgrade). Downgrade para free cancela a cobrança recorrente. */
export async function changePlan(userId: string, plan: Plan) {
  if (!(plan in PLAN_PRICES_CENTS)) throw new DomainError("unknown_plan", `unknown plan ${plan}`);
  await db().query("UPDATE subscriptions SET plan = $2, price_cents = $3 WHERE user_id = $1", [
    userId,
    plan,
    PLAN_PRICES_CENTS[plan],
  ]);
  return { plan, priceCents: PLAN_PRICES_CENTS[plan] };
}
