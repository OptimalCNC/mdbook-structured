# Getting Started

`mdbook-structured` is an external preprocessor for mdBook. Listed JSON and
YAML chapters become structured HTML pages while mdBook retains its normal
book structure, navigation, themes, search, and stock HTML renderer.

## Prerequisites

Install Cargo and mdBook 0.5.x, and start with a book containing a
`SUMMARY.md`.

## Install

```console
cargo install mdbook-structured
```

## Configure

Add both preprocessors to your book's `book.toml`:

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

## Install assets and build

From your book root:

```console
cd path/to/book
mdbook-structured install .
mdbook build
mdbook serve
```

The installer writes starter CSS and JavaScript only; it never edits
`book.toml`. Append the generated paths to `additional-css` and
`additional-js` as directed by the command.

## Add structured chapters

List a source file in `SUMMARY.md` and link to that source path, for example
`[Runtime configuration](config/runtime.yaml)`. mdBook then publishes an
extension-preserving page such as `config/runtime.yaml.html`.

Only chapters listed in `SUMMARY.md` are transformed. Unlisted files remain
ordinary files, and `mdbook test` is not a structured-book acceptance command
in v1 because it uses a different renderer.

## Further reading

- [Configuration](configuration.md)
- [Authoring](authoring.md)
- [Self-contained example](../examples/self-contained/README.md)
- [Design Reference](design/overview.md)
- [Maintainer Guide](maintainers.md)
