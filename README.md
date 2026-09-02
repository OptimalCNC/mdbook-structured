# mdbook-structured

[![CI](https://github.com/OptimalCNC/mdbook-structured/actions/workflows/ci.yml/badge.svg)](https://github.com/OptimalCNC/mdbook-structured/actions/workflows/ci.yml)
[![Publish Docs](https://github.com/OptimalCNC/mdbook-structured/actions/workflows/docs.yml/badge.svg)](https://github.com/OptimalCNC/mdbook-structured/actions/workflows/docs.yml)
[![Release](https://github.com/OptimalCNC/mdbook-structured/actions/workflows/release.yml/badge.svg)](https://github.com/OptimalCNC/mdbook-structured/actions/workflows/release.yml)
[![mdbook-structured version](https://img.shields.io/crates/v/mdbook-structured.svg)](https://crates.io/crates/mdbook-structured)
[![mdbook-structured-core version](https://img.shields.io/crates/v/mdbook-structured-core.svg)](https://crates.io/crates/mdbook-structured-core)

`mdbook-structured` is an external mdBook preprocessor. Its reusable
`mdbook-structured-core` library parses JSON and YAML into a parser-neutral
model and renders readable structured pages in mdBook's stock HTML output.

- Registered `.json`, `.yaml`, and `.yml` chapters become
  extension-preserving HTML.
- Source paths remain authoritative; matched Markdown links are rewritten to
  registered structured pages.
- Starter CSS and JavaScript can be installed without editing `book.toml`.
- Scoped generated-route, parser, and resource-limit diagnostics protect
  registered structured pages.

## Install and use

```console
cargo install mdbook-structured
cd path/to/book
mdbook-structured install .
mdbook build
```

Register the two HTML-only preprocessors in `book.toml`:

```toml
[preprocessor.structured]
command = "mdbook-structured render"
after = ["index"]
before = ["links"]
renderers = ["html"]

[preprocessor.structured-links]
command = "mdbook-structured rewrite-links"
after = ["links"]
renderers = ["html"]
```

Then list `.json`, `.yaml`, or `.yml` chapters in `SUMMARY.md`. See the [Configuration guide](https://optimalcnc.github.io/mdbook-structured/configuration.html) for resource limits and asset registration.

Build-time transformation scope is registered structured chapters, Markdown
links that match those chapters, and the stock HTML renderer; `mdbook test` is
not a structured-book acceptance command in v1.

## Links

- [Published book](https://optimalcnc.github.io/mdbook-structured/)
- [mdbook-structured on docs.rs](https://docs.rs/mdbook-structured)
- [mdbook-structured-core on docs.rs](https://docs.rs/mdbook-structured-core)
- [mdbook-structured on crates.io](https://crates.io/crates/mdbook-structured)
- [mdbook-structured-core on crates.io](https://crates.io/crates/mdbook-structured-core)
- [GitHub repository](https://github.com/OptimalCNC/mdbook-structured)
- [Self-contained example](examples/self-contained/README.md)
- [Design Reference](https://optimalcnc.github.io/mdbook-structured/design/overview.html)
- [Maintainer Guide](https://optimalcnc.github.io/mdbook-structured/maintainers.html)

## Contributing

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
mdbook build .
```

The project is licensed under the MIT license.
