import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { expect, test } from "@playwright/test";

const runtimeUrl = pathToFileURL(
  resolve("target/playwright-book/book/config/runtime.yaml.html"),
).href;
const yamlIndexUrl = pathToFileURL(
  resolve("target/playwright-book/book/config/index.yaml.html"),
).href;

function containerBySummary(page, summary) {
  return page
    .locator("details[data-structured-container] > summary")
    .filter({ hasText: new RegExp(`^${summary}$`) })
    .locator("..");
}

function scalarValue(page, text) {
  return page.locator("[data-structured-value]").filter({ hasText: text });
}

async function expectOpen(locator, open) {
  await expect(locator).toHaveJSProperty("open", open);
}

async function expectRuntimeDefaultPolicy(page) {
  await expectOpen(containerBySummary(page, "Mapping"), true);
  await expectOpen(containerBySummary(page, "service"), true);
  await expectOpen(containerBySummary(page, "ports"), false);
  await expectOpen(containerBySummary(page, "display"), false);
  await expectOpen(containerBySummary(page, "large"), false);
}

test("built structured pages preserve markup policy and keyboard controls", async ({ page }) => {
  await page.goto(runtimeUrl);

  const modelNodes = page.locator("[data-structured-node]");
  const modelContainers = page.locator("details[data-structured-container]");
  const originalSource = page.locator("details[data-structured-original-source]");
  await expect(modelNodes).toHaveCount(16);
  await expect(scalarValue(page, "api")).toHaveText("api");
  await expect(scalarValue(page, "api")).toBeVisible();
  await expect(
    scalarValue(
      page,
      "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-repeat-without-truncation",
    ),
  ).toHaveText(
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-repeat-without-truncation",
  );
  await expect(scalarValue(page, "line one\nline two")).toHaveText("line one\nline two");
  await expect(scalarValue(page, "{{#include missing.md}}")).toHaveText(
    "{{#include missing.md}}",
  );
  await expectRuntimeDefaultPolicy(page);

  const expandAll = page.getByRole("button", { name: "Expand all", exact: true });
  await expandAll.focus();
  await expect(expandAll).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("details[data-structured-container][open]")).toHaveCount(5);
  await expectOpen(originalSource, false);
  await expect(
    scalarValue(
      page,
      "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-repeat-without-truncation",
    ),
  ).toBeVisible();
  await expect(scalarValue(page, "line one\nline two")).toBeVisible();
  await expect(scalarValue(page, "{{#include missing.md}}")).toBeVisible();

  const collapseAll = page.getByRole("button", { name: "Collapse all", exact: true });
  await collapseAll.focus();
  await expect(collapseAll).toBeFocused();
  await page.keyboard.press("Space");
  await expect(page.locator("details[data-structured-container][open]")).toHaveCount(0);
  await expectOpen(originalSource, false);

  await page.goto(yamlIndexUrl);
  const yamlRoot = page.locator(
    "section.structured-document > details[data-structured-node=mapping]",
  );
  const yamlScalars = page.locator(
    "section.structured-document [data-structured-node=string]",
  );
  await expect(yamlRoot).toHaveCount(1);
  await expectOpen(yamlRoot, true);
  await expect(yamlScalars).toHaveCount(2);
  await expect(yamlScalars.nth(0)).toBeVisible();
  await expect(yamlScalars.nth(1)).toBeVisible();

  await page.goto(runtimeUrl);
  await page.reload();
  await expect(modelContainers).toHaveCount(5);
  await expectRuntimeDefaultPolicy(page);
  await expectOpen(originalSource, false);
});
