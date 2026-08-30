import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { expect, test } from "@playwright/test";

const script = await readFile(
  resolve("crates/mdbook-structured/assets/mdbook-structured.js"),
  "utf8",
);

const markup = `
  <section class="structured-document" id="first-document">
    <button type="button" data-structured-action="expand-all">Expand all</button>
    <button type="button" data-structured-action="collapse-all">Collapse all</button>
    <details data-structured-container id="first-parent">
      <summary>First parent</summary>
      <details data-structured-container id="first-child" open>
        <summary>First child</summary>
      </details>
    </details>
    <section class="structured-document" id="nested-document">
      <button type="button" data-structured-action="expand-all">Expand all</button>
      <button type="button" data-structured-action="collapse-all">Collapse all</button>
      <details data-structured-container id="nested-parent">
        <summary>Nested parent</summary>
      </details>
      <details data-structured-original-source id="nested-original" open>
        <summary>Original source</summary>
      </details>
    </section>
    <details data-structured-original-source id="first-original">
      <summary>Original source</summary>
    </details>
  </section>
  <section class="structured-document" id="second-document">
    <button type="button" data-structured-action="expand-all">Expand all</button>
    <button type="button" data-structured-action="collapse-all">Collapse all</button>
    <details data-structured-container id="second-parent" open>
      <summary>Second parent</summary>
      <details data-structured-container id="second-child">
        <summary>Second child</summary>
      </details>
    </details>
    <details data-structured-original-source id="second-original" open>
      <summary>Original source</summary>
    </details>
  </section>
`;

async function disclosureStates(page) {
  return page.locator("details").evaluateAll((details) =>
    details.map(({ id, open }) => ({ id, open })),
  );
}

test("action hooks change only model containers in their structured document", async ({ page }) => {
  await page.setContent(markup);
  await page.addScriptTag({ content: script });

  await page.locator("#first-document > [data-structured-action=expand-all]").click();
  expect(await disclosureStates(page)).toEqual([
    { id: "first-parent", open: true },
    { id: "first-child", open: true },
    { id: "nested-parent", open: false },
    { id: "nested-original", open: true },
    { id: "first-original", open: false },
    { id: "second-parent", open: true },
    { id: "second-child", open: false },
    { id: "second-original", open: true },
  ]);

  await page.locator("#first-document > [data-structured-action=collapse-all]").click();
  expect(await disclosureStates(page)).toEqual([
    { id: "first-parent", open: false },
    { id: "first-child", open: false },
    { id: "nested-parent", open: false },
    { id: "nested-original", open: true },
    { id: "first-original", open: false },
    { id: "second-parent", open: true },
    { id: "second-child", open: false },
    { id: "second-original", open: true },
  ]);

  await page.setContent(markup);
  expect(await disclosureStates(page)).toEqual([
    { id: "first-parent", open: false },
    { id: "first-child", open: true },
    { id: "nested-parent", open: false },
    { id: "nested-original", open: true },
    { id: "first-original", open: false },
    { id: "second-parent", open: true },
    { id: "second-child", open: false },
    { id: "second-original", open: true },
  ]);
});
