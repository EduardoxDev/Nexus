import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["test/**/*.test.ts"],
    testTimeout: 30_000,
    // A preparação indexa a fixture com a CLI real (o antivírus pode atrasar a primeira execução).
    hookTimeout: 60_000,
  },
});
