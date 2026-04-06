import { defineConfig } from "@playwright/test";

const baseURL = "http://127.0.0.1:18080";

export default defineConfig({
  testDir: "./tests/e2e",
  fullyParallel: false,
  retries: process.env.CI ? 2 : 0,
  use: {
    baseURL,
    trace: "on-first-retry",
  },
  webServer: [
    {
      command: "node ../scripts/e2e-upstream.mjs",
      url: "http://127.0.0.1:19081/healthz",
      reuseExistingServer: !process.env.CI,
      timeout: 60_000,
    },
    {
      command: "bash ../scripts/e2e-backend.sh",
      url: `${baseURL}/healthz`,
      reuseExistingServer: !process.env.CI,
      timeout: 120_000,
    },
  ],
});
