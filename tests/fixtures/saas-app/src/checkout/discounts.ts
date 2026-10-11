import { DomainError } from "../utils/errors";
import { percentOf, type Cents } from "../utils/money";

export interface Coupon {
  code: string;
  percentOff: number;
  expiresAt: Date;
}

const coupons = new Map<string, Coupon>();

/** Aplica um cupom de desconto ao subtotal; cupons expirados são recusados. */
export function applyCoupon(subtotal: Cents, code: string, now = new Date()): Cents {
  const coupon = coupons.get(code.toUpperCase());
  if (!coupon) throw new DomainError("coupon_not_found", "cupom inexistente");
  if (coupon.expiresAt < now) throw new DomainError("coupon_expired", "cupom expirado");
  return subtotal - percentOf(subtotal, coupon.percentOff);
}

export function registerCoupon(coupon: Coupon): void {
  coupons.set(coupon.code.toUpperCase(), coupon);
}
