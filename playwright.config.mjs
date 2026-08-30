import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/browser",
  globalSetup: "./tests/browser/prepare-fixture.mjs",
  projects: [
    {
      name: "chromium",
      use: { browserName: "chromium" },
    },
  ],
});
