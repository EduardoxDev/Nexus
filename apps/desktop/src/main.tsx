import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "@/app/App";
import { Providers } from "@/app/providers";
// Inter embutida (sem rede; a CSP só permite fontes do próprio app).
import "@fontsource-variable/inter/wght.css";
import "@/styles/globals.css";

const root = document.getElementById("root");
if (!root) throw new Error("elemento #root ausente");

createRoot(root).render(
  <StrictMode>
    <Providers>
      <App />
    </Providers>
  </StrictMode>,
);
