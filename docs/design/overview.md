# Design Reference

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
the exact lowercase extensions `.json`, `.yaml`, and `.yml`. A chapter is a
registered structured chapter only when mdBook has loaded it into the received
`Book` and its `Chapter.source_path` has one of those extensions. The
implementation does not enumerate the source tree to discover chapters.

At build time, the preprocessor has two semantic operations:

1. transform a registered structured chapter and project its generated route;
2. rewrite an authored Markdown link that resolves to a registered
   structured chapter.

The received `Book` is a transport container for finding inputs to those
operations. A chapter that does not pass the registration gate never enters
structured transformation. During link extraction, a source chapter supplies
only authored bytes and a relative base; only a parser-confirmed link
destination that matches the structured target index enters link rewriting.
Any candidate rejected by its gate is outside the preprocessor's semantic
awareness: it is not validated, routed, diagnosed, or otherwise handled by
this module.

V1 includes:

- JSON and YAML parsing through selected parser libraries;
- a parser-neutral `StructuredDocument` model with source provenance;
- build-time semantic HTML rendering and progressive JavaScript enhancement;
- rewriting of authored Markdown links that resolve to registered
  structured chapters;
- diagnostics scoped to registered structured chapters, their generated
  routes, and matched structured references;
- an installer that generates starter CSS and JavaScript without editing
  `book.toml`; and
- tests that verify semantic behavior at the parser, renderer, preprocessor,
  and installer seams.

V1 does not include a custom mdBook backend, a fork of mdBook, JSON/YAML
parsers in mdBook core, implicit file discovery, non-HTML renderer support,
schema-aware or domain-specific views, generated JSON Pointer anchors,
persistent expansion state, print-specific styling, automatic configuration
file edits, or special presentation for non-displayable control characters.

The ownership split follows the same admission boundary:

- mdBook core owns book structure, source loading and containment, processing
  order, ordinary chapter routes and links, static files, final URL
  generation, and rendering infrastructure;
- mdbook-structured-core owns parser adapters, lossless model projection,
  diagnostics, and structured-page rendering; and
- mdbook-structured owns the mdBook subprocess protocol, the structured
  chapter admission gate, the structured logical-path shim, the structured
  target index, matched-reference rewriting, and the install command.

This keeps the external seam small while allowing future format adapters and
renderers to evolve without coupling their parser types to mdBook. The
[authoring contract](authoring.md) starts from the book author's point of
view; the [processing architecture](architecture.md) then explains how the
components cooperate.
