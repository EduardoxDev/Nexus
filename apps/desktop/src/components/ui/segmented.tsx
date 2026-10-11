import { RadioGroup } from "radix-ui";
import type { LucideIcon } from "lucide-react";

import { cn } from "@/lib/cn";

export interface SegmentedOption<T extends string> {
  value: T;
  label: string;
  icon?: LucideIcon;
}

interface SegmentedProps<T extends string> {
  value: T;
  options: readonly SegmentedOption<T>[];
  onValueChange: (value: T) => void;
  "aria-label": string;
  disabled?: boolean;
}

/** Controle segmentado acessível (radiogroup com navegação por setas). */
export function Segmented<T extends string>({
  value,
  options,
  onValueChange,
  disabled,
  ...aria
}: SegmentedProps<T>) {
  return (
    <RadioGroup.Root
      value={value}
      onValueChange={(v) => {
        const option = options.find((o) => o.value === v);
        if (option) onValueChange(option.value);
      }}
      orientation="horizontal"
      className="inline-flex rounded-md border border-border bg-surface-2 p-0.5"
      {...(disabled === undefined ? {} : { disabled })}
      aria-label={aria["aria-label"]}
    >
      {options.map((option) => {
        const Icon = option.icon;
        return (
          <RadioGroup.Item
            key={option.value}
            value={option.value}
            className={cn(
              "inline-flex h-7 items-center gap-1.5 rounded-[5px] px-3 text-sm text-fg-muted transition-colors",
              "hover:text-fg data-[state=checked]:bg-surface-1 data-[state=checked]:text-fg data-[state=checked]:shadow-sm",
              "dark:data-[state=checked]:bg-surface-3",
            )}
          >
            {Icon && <Icon className="size-3.5" aria-hidden="true" />}
            {option.label}
          </RadioGroup.Item>
        );
      })}
    </RadioGroup.Root>
  );
}
