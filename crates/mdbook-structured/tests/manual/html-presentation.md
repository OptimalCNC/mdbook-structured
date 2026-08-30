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

- Confirm compact key/value and container alignment without overlap.
- Confirm hover feedback is restrained and keyboard focus is clearly visible.
- Confirm container disclosures use native twisties.
- Confirm the root and `service` containers start open, while `ports`, `display`, and `large` start closed.
- Confirm the empty key and empty string use an explicit empty marker.
- Confirm the complete long value wraps without truncation and the complete multiline value remains readable.
- Activate Expand all and Collapse all by keyboard, and confirm they affect the structured model containers but not Original source.
- Open Original source and confirm its format-labelled YAML source is visible and exactly matches the fixture source.
- Disable JavaScript, reload the page, and confirm the structured content remains readable and native disclosure controls still operate.
