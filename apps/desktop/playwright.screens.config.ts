import { defineConfig } from "@playwright/test";

/** Capturas do README (`pnpm screens`): mesmo app real via CDP do E2E, fora da suíte de testes. */
export default defineConfig({
  testDir: "./e2e-screens",
  fullyParallel: false,
  workers: 1,
  timeout: 180_000,
  expect: { timeout: 15_000 },
  reporter: "list",
});
