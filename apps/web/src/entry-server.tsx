import { StrictMode } from "react";
import { renderToString } from "react-dom/server";

import { App } from "./App";

/** HTML estático da página (usado por scripts/prerender.mjs no build). */
export function render(): string {
  return renderToString(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
