import Stripe from "stripe";

import { loadConfig } from "../config";
import { DomainError } from "../utils/errors";

let client: Stripe | null = null;

function stripe(): Stripe {
  client ??= new Stripe(loadConfig().stripeSecretKey);
  return client;
}

/** Cria uma cobrança no Stripe. Valores em centavos (ADR 0002). */
export async function createCharge(amountCents: number, paymentToken: string): Promise<string> {
  if (amountCents <= 0) throw new DomainError("invalid_amount", "amount must be positive");
  const intent = await stripe().paymentIntents.create({
    amount: amountCents,
    currency: "brl",
    payment_method: paymentToken,
    confirm: true,
  });
  return intent.id;
}
