/** Valores monetários são inteiros em centavos (ADR 0002). */
export type Cents = number;

export function formatCents(value: Cents, currency = "BRL"): string {
  return new Intl.NumberFormat("pt-BR", { style: "currency", currency }).format(value / 100);
}

export function percentOf(value: Cents, percent: number): Cents {
  return Math.round((value * percent) / 100);
}
