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
    .filter({
      has: page.locator("[data-structured-label]", {
        hasText: new RegExp(`^${summary}$`),
      }),
    })
    .locator("..");
}

function scalarValue(page, text) {
  return page.locator("[data-structured-value]").filter({ hasText: text });
}

async function expectOpen(locator, open) {
  await expect(locator).toHaveJSProperty("open", open);
}

async function openContainer(locator) {
  if (!(await locator.evaluate((element) => element.open))) {
    await locator.locator(":scope > summary").click();
  }
  await expectOpen(locator, true);
}

async function renderedLabelStarts(locator) {
  return locator.evaluateAll((elements) =>
    elements.map((element) => element.getBoundingClientRect().x),
  );
}

function expectAligned(starts) {
  expect(starts.length).toBeGreaterThan(0);
  expect(Math.max(...starts) - Math.min(...starts)).toBeLessThan(0.5);
}

async function expectRuntimeDefaultPolicy(page) {
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

  const service = containerBySummary(page, "service");
  const serviceNameLabel = service.locator(
    ':scope > [data-structured-node="string"] > [data-structured-label="key"]',
  ).first();
  const servicePortsLabel = service.locator(
    ':scope > details[data-structured-container] > summary > [data-structured-label="key"]',
  ).first();
  const serviceSummaryLabel = service.locator(
    ':scope > summary > [data-structured-label="key"]',
  );
  const [serviceNameStart, servicePortsStart] = await Promise.all([
    serviceNameLabel.evaluate((element) => element.getBoundingClientRect().x),
    servicePortsLabel.evaluate((element) => element.getBoundingClientRect().x),
  ]);
  expect(Math.abs(serviceNameStart - servicePortsStart)).toBeLessThan(0.5);

  const serviceSummaryStart = await serviceSummaryLabel.evaluate(
    (element) => element.getBoundingClientRect().x,
  );
  const structuralIndent = serviceNameStart - serviceSummaryStart;
  expect(structuralIndent).toBeGreaterThan(0);

  const runtimeRoot = page.locator(
    "section.structured-document > div[data-structured-node=mapping]",
  );
  const runtimeRootContainerLabels = runtimeRoot.locator(
    ':scope > details[data-structured-container] > summary > [data-structured-label="key"]',
  );
  const runtimeRootScalarLabels = runtimeRoot.locator(
    ':scope > [data-structured-node="string"] > [data-structured-label="key"]',
  );
  await expect(runtimeRootContainerLabels).toHaveCount(2);
  await expect(runtimeRootScalarLabels).toHaveCount(1);
  expectAligned([
    ...(await renderedLabelStarts(runtimeRootContainerLabels)),
    ...(await renderedLabelStarts(runtimeRootScalarLabels)),
  ]);

  const ports = containerBySummary(page, "ports");
  const display = containerBySummary(page, "display");
  await openContainer(service);
  await openContainer(ports);
  await openContainer(display);

  const nestedAllScalarGroups = [
    {
      container: display,
      labels: display.locator(
        ':scope > [data-structured-node="string"] > [data-structured-label="key"]',
      ),
    },
    {
      container: ports,
      labels: ports.locator(
        ':scope > [data-structured-node="number"] > [data-structured-label="index"]',
      ),
    },
  ];
  for (const { container, labels } of nestedAllScalarGroups) {
    const starts = await renderedLabelStarts(labels);
    expectAligned(starts);
    const rowStart = await labels.first().evaluate(
      (element) => element.parentElement.getBoundingClientRect().x,
    );
    expect(Math.abs(starts[0] - (rowStart + structuralIndent))).toBeLessThan(0.5);
  }

  const markerEscapesDocument = await page.evaluate(() => {
    const section = document.querySelector("section.structured-document");
    const summary = document.querySelector(
      "details[data-structured-container] > summary",
    );
    if (!section || !summary) {
      throw new Error("structured section and container summary are required");
    }
    const sectionBox = section.getBoundingClientRect();
    const summaryBox = summary.getBoundingClientRect();
    return document.elementFromPoint(
      Math.ceil(sectionBox.x) - 1,
      summaryBox.y + summaryBox.height / 2,
    ) === summary;
  });
  expect(markerEscapesDocument).toBe(false);

  const expandAll = page.getByRole("button", { name: "Expand all", exact: true });
  await expandAll.focus();
  await expect(expandAll).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("details[data-structured-container][open]")).toHaveCount(4);
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
    "section.structured-document > div[data-structured-node=mapping]",
  );
  const yamlScalars = page.locator(
    "section.structured-document [data-structured-node=string]",
  );
  await expect(yamlRoot).toHaveCount(1);
  await expect(yamlRoot).not.toHaveAttribute("data-structured-container");
  await expect(yamlRoot.locator(":scope > summary")).toHaveCount(0);
  await expect(yamlScalars).toHaveCount(2);
  await expect(yamlScalars.nth(0)).toBeVisible();
  await expect(yamlScalars.nth(1)).toBeVisible();

  const yamlLabels = yamlRoot.locator(
    ':scope > [data-structured-node="string"] > [data-structured-label="key"]',
  );
  await expect(yamlLabels).toHaveCount(2);
  const yamlLabelStarts = await renderedLabelStarts(yamlLabels);
  expectAligned(yamlLabelStarts);
  const [yamlSectionStart, yamlRootStart] = await Promise.all([
    page
      .locator("section.structured-document")
      .evaluate((element) => element.getBoundingClientRect().x),
    yamlRoot.evaluate((element) => element.getBoundingClientRect().x),
  ]);
  expect(Math.abs(yamlRootStart - yamlSectionStart)).toBeLessThan(0.5);
  expect(Math.abs(yamlLabelStarts[0] - yamlSectionStart)).toBeLessThan(0.5);

  await page.goto(runtimeUrl);
  await page.reload();
  await expect(modelContainers).toHaveCount(4);
  await expectRuntimeDefaultPolicy(page);
  await expectOpen(originalSource, false);
});

test("hard-break markers distinguish authored breaks from soft wrapping", async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 800 });
  await page.goto(runtimeUrl);
  await page.getByRole("button", { name: "Expand all", exact: true }).click();

  const longValue = scalarValue(
    page,
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-repeat-without-truncation",
  );
  const multilineValue = scalarValue(page, "line one\nline two");
  await expect(longValue.locator("[data-structured-line-break]")).toHaveCount(0);
  await expect(multilineValue.locator("[data-structured-line-break]")).toHaveCount(1);

  const visualLineCount = await longValue.evaluate((element) => {
    const range = document.createRange();
    range.selectNodeContents(element);
    return new Set(Array.from(range.getClientRects(), ({ top }) => Math.round(top))).size;
  });
  expect(visualLineCount).toBeGreaterThan(1);

  const marker = multilineValue.locator("[data-structured-line-break]");
  await expect(marker).toHaveAttribute("aria-hidden", "true");
  const markerStyle = await marker.evaluate((element) => {
    const style = getComputedStyle(element, "::after");
    return { content: style.content, userSelect: getComputedStyle(element).userSelect };
  });
  expect(markerStyle.content).toContain("↵");
  expect(markerStyle.userSelect).toBe("none");

  const selectedText = await multilineValue.evaluate((element) => {
    const selection = window.getSelection();
    const range = document.createRange();
    range.selectNodeContents(element);
    selection.removeAllRanges();
    selection.addRange(range);
    const text = selection.toString();
    selection.removeAllRanges();
    return text;
  });
  expect(selectedText).toBe("line one\nline two");
  await expect(
    page.locator(
      "details[data-structured-original-source] [data-structured-line-break]",
    ),
  ).toHaveCount(0);
});

for (const width of [1280, 375]) {
  test(`nested fields indent from their parent content at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    for (const url of [
      runtimeUrl,
      pathToFileURL(resolve("target/playwright-book/book/lists.yaml.html")).href,
    ]) {
      await page.goto(url);
      await page.getByRole("button", { name: "Expand all", exact: true }).click();
      const groups = await page.locator(
        "details[data-structured-container][data-structured-group]",
      ).evaluateAll((containers) => {
        function rowContent(row) {
          return row.querySelector(
            ":scope > [data-structured-label], :scope > summary > [data-structured-label], :scope > summary [data-structured-preview]",
          );
        }
        return containers.map((container) => {
          const parent = rowContent(container);
          const children = Array.from(container.children)
            .filter((child) => child.hasAttribute("data-structured-node"))
            .map(rowContent);
          return {
            label: parent.textContent,
            parentStart: parent.getBoundingClientRect().x,
            childStarts: children.map((child) => child.getBoundingClientRect().x),
          };
        });
      });
      expect(groups.length).toBeGreaterThan(0);
      for (const { label, parentStart, childStarts } of groups) {
        expectAligned(childStarts);
        for (const start of childStarts) {
          expect(start, `children of ${label}`).toBeGreaterThan(parentStart);
        }
      }
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    }
  });

  test(`list previews preserve readable summaries and complete values at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto(pathToFileURL(resolve("target/playwright-book/book/lists.yaml.html")).href);
    const entries = containerBySummary(page, "entries");
    const items = entries.locator(":scope > details[data-structured-container]");
    const first = items.first();
    const summary = first.locator(":scope > summary");
    await expect(entries.locator(":scope > summary > [data-structured-count]")).toHaveText("3 items");
    await expectOpen(first, false);
    await expect(summary.locator("[data-structured-preview-field]")).toHaveText([
      "name: search",
      "enabled: true",
    ]);
    await expect(summary.locator("[data-structured-preview]")).toBeVisible();
    await expect(summary.locator("[data-structured-preview-more]")).toBeVisible();
    await expect(summary.locator('[data-structured-label="index"]')).toHaveText("0");

    const description = first.locator(":scope > [data-structured-node] > [data-structured-value]")
      .filter({ hasText: "Search across documentation." });
    await expect(description).not.toBeVisible();
    await summary.focus();
    await page.keyboard.press("Enter");
    await expect(description).toBeVisible();
    const settings = containerBySummary(page, "settings");
    await openContainer(settings);
    await openContainer(containerBySummary(page, "nested"));
    await expect(settings.locator("[data-structured-preview-field]")).toHaveText(["name: leaf"]);

    const mixed = containerBySummary(page, "mixed");
    const starts = await renderedLabelStarts(mixed.locator(
      ':scope > [data-structured-node="string"] > [data-structured-label="index"], :scope > details > summary [data-structured-preview]',
    ));
    expectAligned(starts);
    const boxes = await summary.evaluate((element) => {
      const preview = element.querySelector("[data-structured-preview]");
      const index = element.querySelector('[data-structured-label="index"]');
      return {
        previewRight: preview.getBoundingClientRect().right,
        indexLeft: index.getBoundingClientRect().left,
        indexRight: index.getBoundingClientRect().right,
        summaryRight: element.getBoundingClientRect().right,
      };
    });
    expect(boxes.indexLeft).toBeGreaterThan(boxes.previewRight);
    expect(boxes.indexRight).toBeLessThanOrEqual(boxes.summaryRight);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);

    const longName = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-notifications\nsecond line";
    const previewName = items.nth(2).locator('[data-structured-preview-value="string"]');
    await expect(previewName).toHaveText(longName);
    expect(await previewName.evaluate((element) => element.scrollWidth > element.clientWidth)).toBe(true);
    await openContainer(items.nth(2));
    const fullName = items.nth(2).locator(':scope > [data-structured-node="string"] > [data-structured-value]').first();
    await expect(fullName).toBeVisible();
    await expect(fullName).toHaveText(longName);
  });
}

test("list previews and disclosures work without JavaScript", async ({ browser }) => {
  const context = await browser.newContext({ javaScriptEnabled: false });
  try {
    const page = await context.newPage();
    await page.goto(pathToFileURL(resolve("target/playwright-book/book/lists.yaml.html")).href);
    const first = containerBySummary(page, "entries").locator(":scope > details").first();
    await expect(first.locator("[data-structured-preview]").first()).toBeVisible();
    await first.locator(":scope > summary").click();
    await expect(first.locator(":scope > [data-structured-node] > [data-structured-value]")
      .filter({ hasText: "Search across documentation." })).toBeVisible();
  } finally {
    await context.close();
  }
});
