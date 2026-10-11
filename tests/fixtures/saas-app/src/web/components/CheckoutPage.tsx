import { cartSubtotal, type CartItem } from "../../checkout/cart";
import { PriceTag } from "./PriceTag";

/** Página de checkout: resumo do carrinho e botão de pagamento. */
export function CheckoutPage({ items }: { items: CartItem[] }) {
  async function pay() {
    await fetch("/api/orders/checkout", { method: "POST", body: JSON.stringify({ items, zip: "01001000" }) });
  }
  return (
    <section>
      <h1>Finalizar compra</h1>
      <PriceTag cents={cartSubtotal(items)} />
      <button onClick={() => void pay()}>Pagar</button>
    </section>
  );
}
