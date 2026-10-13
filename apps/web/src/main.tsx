import { StrictMode } from "react";
import { createRoot, hydrateRoot } from "react-dom/client";

import "@fontsource-variable/inter/wght.css";

import { App } from "./App";
import "./styles.css";

const root = document.getElementById("root");
if (!root) throw new Error("#root ausente");
const app = (
  <StrictMode>
    <App />
  </StrictMode>
);
// No build, o HTML já vem pré-renderizado (scripts/prerender.mjs): o React só assume a página.
// No dev só há o comentário <!--app-->: aí não há o que hidratar.
if (root.firstElementChild) hydrateRoot(root, app);
else createRoot(root).render(app);
