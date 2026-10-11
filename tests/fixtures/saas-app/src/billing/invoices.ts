import { db } from "../db/client";
import type { Cents } from "../utils/money";

export interface Invoice {
  number: string;
  orderId: string;
  totalCents: Cents;
}

/** Número sequencial no formato ACME-AAAA-000123. */
export function invoiceNumber(sequence: number, year = new Date().getFullYear()): string {
  return `ACME-${year}-${String(sequence).padStart(6, "0")}`;
}

export async function createInvoice(orderId: string, totalCents: Cents): Promise<Invoice> {
  const { rows } = await db().query("SELECT nextval('invoice_seq') AS seq");
  const invoice = { number: invoiceNumber(Number(rows[0].seq)), orderId, totalCents };
  await db().query("INSERT INTO invoices (number, order_id, total_cents) VALUES ($1, $2, $3)", [
    invoice.number,
    orderId,
    totalCents,
  ]);
  return invoice;
}
