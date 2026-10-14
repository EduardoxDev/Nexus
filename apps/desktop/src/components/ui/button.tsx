import { cva, type VariantProps } from "class-variance-authority";
import { Slot } from "radix-ui";
import type { ComponentProps } from "react";

import { cn } from "@/lib/cn";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-1.5 whitespace-nowrap rounded-md text-sm font-medium transition-[color,background-color,border-color,box-shadow,transform] duration-[var(--duration-fast)] active:scale-[0.97] disabled:pointer-events-none disabled:opacity-50 [&_svg]:size-4 [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        // Azul com leve degradê e brilho no hover (como o botão de download do site).
        primary:
          "bg-[linear-gradient(180deg,oklch(0.64_0.19_268),var(--accent))] text-accent-fg [box-shadow:inset_0_1px_0_oklch(1_0_0/0.22),0_1px_2px_oklch(0.3_0.15_266/0.4)] hover:[box-shadow:inset_0_1px_0_oklch(1_0_0/0.25),0_8px_24px_-8px_var(--accent)] hover:brightness-110",
        secondary:
          "border border-border bg-surface-2 text-fg shadow-card hover:border-border-strong hover:bg-surface-3",
        ghost: "text-fg-muted hover:bg-surface-2 hover:text-fg",
        danger: "bg-danger text-accent-fg shadow-card hover:opacity-90",
      },
      size: {
        sm: "h-7 px-2.5",
        md: "h-8 px-3",
        lg: "h-10 rounded-lg px-4",
        icon: "size-8",
        "icon-sm": "size-7",
      },
    },
    defaultVariants: { variant: "secondary", size: "md" },
  },
);

export interface ButtonProps extends ComponentProps<"button">, VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

export function Button({ className, variant, size, asChild = false, ...props }: ButtonProps) {
  const Comp = asChild ? Slot.Root : "button";
  return (
    <Comp
      data-slot="button"
      className={cn(buttonVariants({ variant, size }), className)}
      {...(asChild ? {} : { type: "button" })}
      {...props}
    />
  );
}
