import { StrictMode } from "react";
import { renderToString } from "react-dom/server";

import { App } from "./App";
import { DOCS, HOME_TITLE, PATHS } from "./routes";
import { site } from "./site";

export { PATHS };

/** HTML estático de uma página, com título e descrição (usado por scripts/prerender.mjs). */
export function render(path = "/"): { html: string; title: string; description: string } {
  const doc = DOCS.find((d) => d.path === path);
  return {
    html: renderToString(
      <StrictMode>
        <App path={path} />
      </StrictMode>,
    ),
    title: doc ? `${doc.title} — NEXUS` : HOME_TITLE,
    description: doc?.description ?? site.description,
  };
}
