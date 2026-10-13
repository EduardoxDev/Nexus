import { ChevronDown } from "lucide-react";
import { Accordion as A } from "radix-ui";
import type { ComponentProps } from "react";

import { cn } from "@/lib/cn";

export const Accordion = A.Root;

export function AccordionItem({ className, ...props }: ComponentProps<typeof A.Item>) {
  return <A.Item className={cn("border-b border-border last:border-b-0", className)} {...props} />;
}

export function AccordionTrigger({
  className,
  children,
  ...props
}: ComponentProps<typeof A.Trigger>) {
  return (
    <A.Header className="flex">
      <A.Trigger
        className={cn(
          "group flex flex-1 items-center justify-between gap-4 py-5 text-left text-[15px] font-medium transition-colors hover:text-primary",
          className,
        )}
        {...props}
      >
        {children}
        <ChevronDown
          className="size-4 shrink-0 text-muted-foreground transition-transform duration-300 group-data-[state=open]:rotate-180"
          aria-hidden="true"
        />
      </A.Trigger>
    </A.Header>
  );
}

export function AccordionContent({
  className,
  children,
  ...props
}: ComponentProps<typeof A.Content>) {
  return (
    <A.Content
      className="overflow-hidden text-sm data-[state=closed]:animate-accordion-up data-[state=open]:animate-accordion-down"
      {...props}
    >
      <div className={cn("pb-5 leading-relaxed text-muted-foreground", className)}>{children}</div>
    </A.Content>
  );
}
