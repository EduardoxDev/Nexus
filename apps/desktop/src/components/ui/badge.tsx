import { cva, type VariantProps } from "class-variance-authority";
import type { ComponentProps } from "react";

import { cn } from "@/lib/cn";

const badgeVariants = cva(
  "inline-flex items-center gap-1 rounded-full px-2 py-px text-xs font-medium whitespace-nowrap ring-1 ring-inset",
  {
    variants: {
      tone: {
        neutral: "bg-surface-2 text-fg-muted ring-border",
        accent: "bg-accent-subtle text-accent-text ring-accent/25",
        success: "bg-success-subtle text-success ring-success/25",
        warning: "bg-warning-subtle text-warning ring-warning/25",
        danger: "bg-danger-subtle text-danger ring-danger/25",
        info: "bg-info-subtle text-info ring-info/25",
      },
    },
    defaultVariants: { tone: "neutral" },
  },
);

export interface BadgeProps extends ComponentProps<"span">, VariantProps<typeof badgeVariants> {}

export function Badge({ className, tone, ...props }: BadgeProps) {
  return <span className={cn(badgeVariants({ tone }), className)} {...props} />;
}
