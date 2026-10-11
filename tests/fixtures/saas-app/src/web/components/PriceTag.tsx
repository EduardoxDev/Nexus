import { formatCents } from "../../utils/money";

export function PriceTag({ cents }: { cents: number }) {
  return <span className="price">{formatCents(cents)}</span>;
}
