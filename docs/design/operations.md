# Configuration and Operations

The v1 executable exposes `render`, `rewrite-links`, and `install`.

## Preprocessor protocol

`render` and `rewrite-links` use the standard mdBook external-preprocessor
protocol: a `[PreprocessorContext, Book]` JSON tuple enters on stdin, exactly
one transformed `Book` JSON value leaves on stdout, and human diagnostics go
only to stderr.

Both commands are restricted to the stock `html` renderer. Their nested
capability forms are:

```console
mdbook-structured render supports html
mdbook-structured rewrite-links supports html
```

Those forms succeed; other renderer names are unsupported. During a normal
run, either command rejects a malformed protocol value or an unexpected
renderer instead of silently passing through a partially processed book.

## mdBook registration

The recommended complete configuration is:

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

The ordering implements the [processing pipeline](architecture.md): `render`
converts registered JSON/YAML chapters, assigns their source-extension route
shims, and does so before helper expansion. `rewrite-links` runs afterward;
parser-confirmed link destinations, including those introduced by includes,
are edited only when they match the structured target index.

Registration is derived only from chapter metadata in the received `Book` and
an exact lowercase `.json`, `.yaml`, or `.yml` `Chapter.source_path`
extension. Each registered structured chapter must parse, render, and project
its generated route successfully. Parser-reported malformed input, duplicate
decoded keys, lossless-projection failures, generated-route errors, and
resource-limit violations stop the build with the registered source identity.
Source location and structured path are included when applicable. There is no
silent fallback that leaves a broken registered source as raw text.

Only a registered structured chapter enters `render`. During `rewrite-links`,
a source chapter supplies authored bytes and a relative base, but only a link
destination matched by the structured target index enters the operation's
semantic domain. mdBook owns book interpretation and routing beyond those
admitted values.

Limits are explicit configuration, not silent truncation. `command`, `after`,
`before`, and `renderers` are mdBook-owned registration keys rather than plugin
options. The same applies to mdBook's standard `optional` key. After those
keys are excluded, an unknown or invalid plugin-specific configuration field
is an error.

Books containing registered structured chapters support `mdbook build` with
the stock HTML renderer. `mdbook test` uses a different renderer and is
unsupported for such books in v1. With the recommended renderer filters,
mdBook skips both
commands during `mdbook test` and applies its default test behavior to the
untransformed structured text, so the result is content-dependent and carries
no tool guarantee. The tool does not remove or replace structured chapters to
create a test-renderer mode.

## Asset installation

```console
mdbook-structured install [DIR]
```

The command writes `mdbook-structured.css` and `mdbook-structured.js` under
`DIR`, which defaults to the current book root. It never reads or modifies
`book.toml`. It prints instructions showing where authors should add the asset
paths in `additional-css` and `additional-js`. When either array already
exists, authors append the printed path rather than replacing existing
entries. Registration remains an author decision.

The installer processes the two files sequentially. It creates an absent
file, leaves a byte-identical existing file unchanged, and stops on the first
different existing file or unexpected write error. Partial side effects are
allowed; rollback and atomic multi-file updates are intentionally out of
scope. The generated assets are starter implementations using mdBook theme
variables and can be edited or replaced by the author.
