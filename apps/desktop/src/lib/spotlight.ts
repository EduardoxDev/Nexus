import { useEffect } from "react";

/**
 * Posição do mouse para os cartões com `data-spotlight` (efeito definido em globals.css).
 * Um só listener para o app inteiro: só o cartão sob o cursor é atualizado.
 */
export function useSpotlight(): void {
  useEffect(() => {
    const onMove = (e: PointerEvent) => {
      const card = (e.target as Element | null)?.closest<HTMLElement>("[data-spotlight]");
      if (!card) return;
      const r = card.getBoundingClientRect();
      card.style.setProperty("--spot-x", `${e.clientX - r.left}px`);
      card.style.setProperty("--spot-y", `${e.clientY - r.top}px`);
    };
    document.addEventListener("pointermove", onMove, { passive: true });
    return () => document.removeEventListener("pointermove", onMove);
  }, []);
}
