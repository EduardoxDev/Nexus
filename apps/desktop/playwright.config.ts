import { defineConfig } from "@playwright/test";

/**
 * E2E contra o app real (executável Tauri + WebView2), conectado via CDP.
 * Não usa navegadores do Playwright; um app por teste, em série.
 */
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  workers: 1,
  timeout: 90_000,
  expect: { timeout: 10_000 },
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : "list",
  use: { trace: "retain-on-failure" },
});
