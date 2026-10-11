import type { Cents } from "../utils/money";

/** Frete por faixa de CEP; acima de R$ 300,00 o frete é grátis. */
export function shippingCost(zip: string, subtotal: Cents): Cents {
  if (subtotal >= 30_000) return 0;
  const region = Number(zip.slice(0, 1));
  return region <= 3 ? 1_500 : 2_900;
}
