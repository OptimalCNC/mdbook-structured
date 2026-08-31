# mdbook-structured Self-Contained Example

This runnable mdBook demonstrates how `mdbook-structured` renders listed JSON
and YAML chapters alongside ordinary Markdown. It includes a complete
`book.toml` and representative source files.

## Prerequisites

Install mdBook 0.5.x and `mdbook-structured`:

```console
cargo install mdbook --version '^0.5'
cargo install mdbook-structured
```

## Prepare, build, and serve

From the repository checkout:

```console
cd examples/self-contained
mdbook-structured install .
mdbook build
mdbook serve
```

The installer creates the starter presentation assets referenced by
`book.toml`. The build output is written to `book/`. While `mdbook serve` is
running, open the URL printed by mdBook to browse the example.

## What the example demonstrates

- `src/config/runtime.yaml` shows mappings, sequences, scalar types, empty
  values, long and multiline strings, literal helper-like text, and the
  original source view.
- `src/mixed-top-level.yaml` alternates scalar and foldable fields at the data
  root so their presentation can be compared on one page.
- `src/config/README.yaml` and `src/config/README.json` become distinct
  extension-qualified pages (`index.yaml.html` and `index.json.html`).
- Links use the exact `config/README.yaml` and `config/README.json` source
  paths because their shared `config/` convenience alias is ambiguous.
- `src/exact/index.md` demonstrates an ordinary Markdown index route, while
  `src/single/README.yaml` demonstrates a unique structured README alias.
- Links in `src/README.md` are authored with source paths and rewritten to the
  corresponding structured pages.

## Generated assets

The example does not track generated presentation assets.
`mdbook-structured install .` creates `mdbook-structured.css` and
`mdbook-structured.js` locally, and `book.toml` registers them through
`output.html.additional-css` and `output.html.additional-js`.
