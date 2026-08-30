import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/browser",
  projects: [
    {
      name: "chromium",
      use: { browserName: "chromium" },
    },
  ],
});
