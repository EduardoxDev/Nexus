import { formatCents, type Cents } from "../utils/money";

export function orderConfirmationTemplate(orderId: string, total: Cents): { subject: string; body: string } {
  return {
    subject: `Pedido ${orderId} confirmado`,
    body: `Recebemos seu pagamento de ${formatCents(total)}. Obrigado pela compra!`,
  };
}
