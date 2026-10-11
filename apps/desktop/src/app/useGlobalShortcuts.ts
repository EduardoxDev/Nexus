import { useEffect } from "react";

import { ROUTES } from "@/app/routes";
import { useUi } from "@/stores/ui";

interface Options {
  onToggleSidebar: () => void;
}

/** Atalhos globais documentados em `t.shortcuts` (e na tela de atalhos, F1). */
export function useGlobalShortcuts({ onToggleSidebar }: Options): void {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const ui = useUi.getState();
      const ctrl = event.ctrlKey || event.metaKey;
      const key = event.key.toLowerCase();

      if (ctrl && key === "k") {
        event.preventDefault();
        ui.setPaletteOpen(!ui.paletteOpen);
        return;
      }
      if (event.key === "F1") {
        event.preventDefault();
        ui.setShortcutsOpen(true);
        return;
      }
      // Os atalhos abaixo não disparam com diálogos abertos.
      if (ui.paletteOpen || ui.shortcutsOpen) return;

      if (ctrl && /^[1-9]$/.test(event.key)) {
        const route = ROUTES[Number(event.key) - 1];
        if (route) {
          event.preventDefault();
          ui.navigate(route.id);
        }
        return;
      }
      if (ctrl && key === ",") {
        event.preventDefault();
        ui.navigate("settings");
        return;
      }
      if (ctrl && key === "b") {
        event.preventDefault();
        onToggleSidebar();
        return;
      }
      if (event.altKey && event.key === "ArrowLeft") {
        event.preventDefault();
        ui.back();
        return;
      }
      if (event.altKey && event.key === "ArrowRight") {
        event.preventDefault();
        ui.forward();
      }
    };

    // Botões "voltar/avançar" do mouse.
    const onMouseUp = (event: MouseEvent) => {
      if (event.button === 3) useUi.getState().back();
      if (event.button === 4) useUi.getState().forward();
    };

    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("mouseup", onMouseUp);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("mouseup", onMouseUp);
    };
  }, [onToggleSidebar]);
}
