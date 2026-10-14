import "@testing-library/jest-dom/vitest";
import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, configure } from "@testing-library/react";
import { afterEach } from "vitest";

import { resetToasts } from "@/stores/toast";
import { resetUi } from "@/stores/ui";

// `findBy*`/`waitFor` esperam 1 s por padrão; com a suíte inteira rodando em paralelo (ou a
// máquina ocupada) a tela às vezes leva mais que isso. Falhas reais continuam falhando.
configure({ asyncUtilTimeout: 5_000 });

// jsdom não implementa estas APIs usadas por Radix/cmdk/motion.
class ResizeObserverStub {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
}
if (!("ResizeObserver" in globalThis)) {
  Object.assign(globalThis, { ResizeObserver: ResizeObserverStub });
}
Object.assign(Element.prototype, {
  scrollIntoView: () => undefined,
  hasPointerCapture: () => false,
  releasePointerCapture: () => undefined,
});

// jsdom não desenha em canvas: sem contexto, o mapa só monta o layout.
HTMLCanvasElement.prototype.getContext = (() =>
  null) as typeof HTMLCanvasElement.prototype.getContext;

if (typeof window.matchMedia !== "function") {
  window.matchMedia = (query: string) =>
    ({
      matches: false,
      media: query,
      onchange: null,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
      addListener: () => undefined,
      removeListener: () => undefined,
      dispatchEvent: () => false,
    }) as MediaQueryList;
}

afterEach(() => {
  cleanup();
  clearMocks();
  resetUi();
  resetToasts();
});
