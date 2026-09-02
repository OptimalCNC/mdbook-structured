# Configuration

Register the renderer and link rewriter in `book.toml`:

```toml
[preprocessor.structured]
command = "mdbook-structured render"
after = ["index"]
before = ["links"]
renderers = ["html"]
max-input-bytes = 1048576
max-nodes = 10000
max-depth = 64
large-container-threshold = 100

[preprocessor.structured-links]
command = "mdbook-structured rewrite-links"
after = ["links"]
renderers = ["html"]
```

The limits are deliberately explicit:

| Option | Default |
| --- | ---: |
| `max-input-bytes` | 1048576 |
| `max-nodes` | 10000 |
| `max-depth` | 64 |
| `large-container-threshold` | 100 |

`render` runs after mdBook's `index` preprocessor and before `links`.
`rewrite-links` runs after `links`; both commands are restricted to `html`.
See [configuration and operations](design/operations.md), [routes](design/routes.md),
and [link rewriting](design/link-rewriting.md) for the normative contracts.

## Assets

Run `mdbook-structured install .` in the book root. Add the printed paths to
`output.html.additional-css` and `output.html.additional-js`; existing arrays
are preserved. The installer never edits `book.toml`.

## What changes at build time

Registered `.json`, `.yaml`, and `.yml` chapters are transformed into
structured HTML, and parser-confirmed Markdown link destinations are rewritten
only when they resolve to one of those registered sources. The source path
remains the registered identity and authoritative file on disk. Objects
admitted by neither operation remain outside the preprocessor's semantic
domain; mdBook owns their handling.

## Diagnostics and troubleshooting

The preprocessor fails fast on malformed protocol input, invalid options, or a
failure concerning a registered structured chapter, its generated route, or a
matched structured reference. Structured diagnostics include the source
filename and location when known.

If a page is missing, verify that its source appears in `SUMMARY.md`, the
renderer is `html`, and both registrations are present. If links do not point
to `.json.html`, `.yaml.html`, or `.yml.html` pages, keep the rewriter after
`links` and link to the source path. For unsupported `mdbook test` behavior,
use `mdbook build` with the stock HTML renderer instead.
