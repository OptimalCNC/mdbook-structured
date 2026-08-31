# Structured HTML presentation manual check

This is a maintained manual procedure for checking the stock mdBook presentation. It is not an automated check or a claim that the visual inspection has already been executed.

## Setup

From the repository root, run:

```bash
npm ci
node --input-type=module -e 'import setup from "./tests/browser/prepare-fixture.mjs"; await setup();'
python3 -m http.server 8000 --directory target/playwright-book/book
```

Open <http://127.0.0.1:8000/config/runtime.yaml.html>.

## Pass/fail record

For each theme and viewport, record `Pass` or `Fail` after completing the checklist below.

| Theme | `1440x900` | `375x800` |
| --- | --- | --- |
| default |  |  |
| light |  |  |
| rust |  |  |
| ayu |  |  |

## Checklist

- Confirm that the directly rendered mixed root and its foldable/scalar
  siblings share one decoded-label column; scalar rows retain the disclosure
  gutter there, and native fold markers stay within the structured-document
  area.
- Open `service`, then verify its mixed `name` and `ports` rows remain aligned.
  Expand `display` (all-scalar mapping) and `ports` (all-scalar sequence): omit
  only their marker gutters while preserving structural nesting indentation
  and one sibling label column. Check the all-scalar root on the
  `config/index.yaml.html` page the same way, and confirm empty containers keep
  their existing disclosure behavior.
- Confirm hover feedback is restrained and keyboard focus is clearly visible.
- Confirm container disclosures use native twisties.
- Confirm the data starts directly with `service`, without a synthetic `Mapping` row; `service` starts open, while `ports`, `display`, and `large` start closed.
- Confirm the empty key and empty string use an explicit empty marker.
- Confirm the complete long value wraps without truncation or a hard-break marker, while each authored break in the multiline value shows a muted `↵` marker.
- Select and copy the multiline value, and confirm the copied text retains its newline without the visible `↵` marker.
- Activate Expand all and Collapse all by keyboard, and confirm they affect the structured model containers but not Original source.
- Open Original source and confirm its format-labelled YAML source is visible and exactly matches the fixture source.
- Disable JavaScript, reload the page, and confirm the structured content remains readable and native disclosure controls still operate.
