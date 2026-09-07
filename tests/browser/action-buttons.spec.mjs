import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { expect, test } from "@playwright/test";

const runtimeUrl = pathToFileURL(
  resolve("target/playwright-book/book/config/runtime.yaml.html"),
).href;

test("icon actions reveal dismissible labels and work by keyboard", async ({ page }) => {
  await page.goto(runtimeUrl);
  const expand = page.getByRole("button", { name: "Expand all", exact: true });
  const collapse = page.getByRole("button", { name: "Collapse all", exact: true });
  const expandLabel = expand.locator("[data-structured-tooltip]");
  const collapseLabel = collapse.locator("[data-structured-tooltip]");
  const containers = page.locator("details[data-structured-container]");
  const original = page.locator("details[data-structured-original-source]");

  await expect(expand.locator("svg")).toBeVisible();
  await expect(collapse.locator("svg")).toBeVisible();
  await expect(expandLabel).toBeHidden();
  await expect(collapseLabel).toBeHidden();
  const initialBounds = await expand.boundingBox();
  await expand.hover();
  await expect(expandLabel).toBeVisible();
  await expect(expandLabel).toHaveText("Expand all");
  expect(await expand.boundingBox()).toEqual(initialBounds);
  await expandLabel.hover();
  await expect(expandLabel).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(expandLabel).toBeHidden();

  await page.mouse.move(0, 0);
  await expand.focus();
  await page.keyboard.press("Tab");
  await expect(collapse).toBeFocused();
  await expect(collapse).toHaveCSS("outline-style", "solid");
  await expect(collapseLabel).toBeVisible();
  await expect(expandLabel).toBeHidden();
  await page.keyboard.press("Escape");
  await expect(collapseLabel).toBeHidden();
  await expect(collapse).toBeFocused();
  await page.keyboard.press("Space");
  await expect(page.locator("details[data-structured-container][open]")).toHaveCount(0);
  await expect(original).toHaveJSProperty("open", false);

  await page.keyboard.press("Shift+Tab");
  await expect(expand).toBeFocused();
  await expect(expandLabel).toBeVisible();
  await page.keyboard.press("Enter");
  await expect(page.locator("details[data-structured-container][open]")).toHaveCount(
    await containers.count(),
  );
  await expect(expandLabel).toBeHidden();
  await expect(expand).toBeFocused();
  await expect(original).toHaveJSProperty("open", false);
  await original.locator("summary").click();
  await collapse.click();
  await expect(original).toHaveJSProperty("open", true);
});

test("button geometry and contrast follow all mdBook themes", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(runtimeUrl);
  const expand = page.getByRole("button", { name: "Expand all", exact: true });
  for (const theme of ["light", "navy", "coal", "ayu", "rust"]) {
    await page.evaluate((theme) => {
      document.documentElement.classList.remove("light", "navy", "coal", "ayu", "rust");
      document.documentElement.classList.add(theme);
    }, theme);
    const appearance = await expand.evaluate((button) => {
      const context = document.createElement("canvas").getContext("2d");
      function luminance(color) {
        context.fillStyle = color;
        context.fillRect(0, 0, 1, 1);
        return [...context.getImageData(0, 0, 1, 1).data]
          .slice(0, 3)
          .map((value) => {
            value /= 255;
            return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
          })
          .reduce((sum, value, index) => sum + value * [0.2126, 0.7152, 0.0722][index], 0);
      }
      function contrast(first, second) {
        const values = [luminance(first), luminance(second)];
        return (Math.max(...values) + 0.05) / (Math.min(...values) + 0.05);
      }
      const style = getComputedStyle(button);
      const tooltip = getComputedStyle(button.querySelector("[data-structured-tooltip]"));
      const bounds = button.getBoundingClientRect();
      const next = button.nextElementSibling.getBoundingClientRect();
      return {
        width: bounds.width,
        height: bounds.height,
        gap: next.left - bounds.right,
        iconContrast: contrast(style.color, style.backgroundColor),
        labelContrast: contrast(tooltip.color, tooltip.backgroundColor),
        focusContrast: contrast(
          style.getPropertyValue("--links"),
          getComputedStyle(document.documentElement).backgroundColor,
        ),
      };
    });
    expect(appearance.width, theme).toBe(32);
    expect(appearance.height, theme).toBe(32);
    expect(appearance.gap, theme).toBe(8);
    expect(appearance.iconContrast, theme).toBeGreaterThanOrEqual(3);
    expect(appearance.labelContrast, theme).toBeGreaterThanOrEqual(4.5);
    expect(appearance.focusContrast, theme).toBeGreaterThanOrEqual(3);
  }
  await expect(expand).toHaveCSS("transition-duration", "0s");
});

for (const width of [320, 390]) {
  test.describe("touch controls at " + width + "px", () => {
    test.use({ viewport: { width, height: 844 }, hasTouch: true, isMobile: true });

    test("keep both actions compact, tappable, and independent of source", async ({ page }) => {
      await page.goto(runtimeUrl);
      const expand = page.getByRole("button", { name: "Expand all", exact: true });
      const collapse = page.getByRole("button", { name: "Collapse all", exact: true });
      const first = await expand.boundingBox();
      const second = await collapse.boundingBox();
      expect(first.width).toBe(44);
      expect(first.height).toBe(44);
      expect(second.width).toBe(44);
      expect(second.height).toBe(44);
      expect(second.y).toBe(first.y);
      expect(second.x - first.x - first.width).toBe(8);
      await expand.tap();
      await expect(page.locator("details[data-structured-container][open]")).toHaveCount(
        await page.locator("details[data-structured-container]").count(),
      );
      await collapse.tap();
      await expect(page.locator("details[data-structured-container][open]")).toHaveCount(0);
      await expect(page.locator("details[data-structured-original-source]")).toHaveJSProperty(
        "open", false,
      );
      await expect(page.locator("[data-structured-tooltip-open]")).toHaveCount(0);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    });
  });
}
