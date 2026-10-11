import { useEffect, useSyncExternalStore } from "react";

import type { MotionPreference, ThemePreference } from "@/lib/bindings";

export type ResolvedTheme = "dark" | "light";

function mediaQuery(query: string): MediaQueryList | null {
  return typeof window !== "undefined" && typeof window.matchMedia === "function"
    ? window.matchMedia(query)
    : null;
}

function useMedia(query: string): boolean {
  return useSyncExternalStore(
    (notify) => {
      const mq = mediaQuery(query);
      mq?.addEventListener("change", notify);
      return () => mq?.removeEventListener("change", notify);
    },
    () => mediaQuery(query)?.matches ?? false,
    () => false,
  );
}

export function resolveTheme(pref: ThemePreference, systemPrefersLight: boolean): ResolvedTheme {
  if (pref === "system") return systemPrefersLight ? "light" : "dark";
  return pref;
}

export function resolveReducedMotion(pref: MotionPreference, systemReduces: boolean): boolean {
  if (pref === "system") return systemReduces;
  return pref === "on";
}

/** Aplica tema e movimento no `<html>` e devolve os valores efetivos. */
export function useApplyAppearance(
  theme: ThemePreference,
  motion: MotionPreference,
): { theme: ResolvedTheme; reducedMotion: boolean } {
  const prefersLight = useMedia("(prefers-color-scheme: light)");
  const prefersReduced = useMedia("(prefers-reduced-motion: reduce)");
  const resolved = resolveTheme(theme, prefersLight);
  const reducedMotion = resolveReducedMotion(motion, prefersReduced);

  useEffect(() => {
    const root = document.documentElement;
    root.classList.toggle("dark", resolved === "dark");
    root.classList.toggle("light", resolved === "light");
    root.dataset.theme = resolved;
    root.dataset.reducedMotion = String(reducedMotion);
  }, [resolved, reducedMotion]);

  return { theme: resolved, reducedMotion };
}
