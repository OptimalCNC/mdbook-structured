# mdbook-structured Design

Status: Approved design for v1

Date: 2026-08-28

mdbook-structured is an external Rust mdBook preprocessor backed by a
reusable, format-extensible library. The library exposes a parser-neutral
model for JSON-compatible structured documents. It renders selected JSON and
YAML source files as readable, styled pages in mdBook's stock HTML output.
The source files remain authoritative on disk, and each generated page remains
part of the ordinary mdBook book: navigation, themes, search, edit links, and
the stock renderer continue to own their usual responsibilities.

The architecture is intentionally general, but the v1 input set is fixed to
`.json`, `.yaml`, and `.yml`. A file is eligible only when mdBook has loaded it
as a chapter listed in `SUMMARY.md`. The implementation does not enumerate the
source tree to discover unlisted chapters.

V1 includes:

- JSON and YAML parsing through established parser libraries;
- a parser-neutral `StructuredDocument` model with source provenance;
- build-time semantic HTML rendering and progressive JavaScript enhancement;
- book-wide Markdown link rewriting for registered structured chapters;
- chapter-route collision preflight and fail-fast diagnostics;
- an installer that generates starter CSS and JavaScript without editing
  `book.toml`; and
- tests that verify semantic behavior at the parser, renderer, preprocessor,
  and installer seams.

V1 does not include a custom mdBook backend, a fork of mdBook, JSON/YAML
parsers in mdBook core, implicit file discovery, non-HTML renderer support,
schema-aware or domain-specific views, generated JSON Pointer anchors,
persistent expansion state, print-specific styling, automatic configuration
file edits, or special presentation for non-displayable control characters.

The ownership split is:

- mdBook core owns book structure, processing order, interpretation of final
  chapter paths, URL generation, and rendering infrastructure;
- mdbook-structured-core owns parser adapters, lossless model projection,
  diagnostics, and structured-page rendering; and
- mdbook-structured owns the mdBook subprocess protocol, chapter selection,
  the structured logical-path shim, book-wide link mapping, and the install
  command.

This keeps the external seam small while allowing future format adapters and
renderers to evolve without coupling their parser types to mdBook. The
[authoring contract](design/authoring.md) starts from the book author's point
of view; the [processing architecture](design/architecture.md) then explains
how the components cooperate.
