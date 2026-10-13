import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

// jsdom não implementa estas APIs usadas pelo motion.
class ObserverStub {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
  takeRecords(): [] {
    return [];
  }
}
Object.assign(globalThis, {
  IntersectionObserver: globalThis.IntersectionObserver ?? ObserverStub,
  ResizeObserver: globalThis.ResizeObserver ?? ObserverStub,
});
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

// jsdom não tem a API de fontes (usada pelo SplitText do React Bits).
if (!("fonts" in document)) {
  Object.defineProperty(document, "fonts", {
    value: {
      status: "loaded",
      ready: Promise.resolve(),
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
    },
  });
}

afterEach(() => cleanup());
