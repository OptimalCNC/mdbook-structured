# Processing Architecture

The stock mdBook pipeline is extended with two registrations of the same
executable:

```text
SUMMARY.md
    |
    v
mdBook loads listed UTF-8 files as Chapter.content
    |
    v
mdBook's index preprocessor
    |
    v
mdbook-structured render
    |  parse eligible sources, project, assign routes, render semantic HTML
    v
mdBook's links preprocessor
    |  expand the normal Markdown helpers/includes
    v
mdbook-structured rewrite-links
    |  resolve Markdown links to registered chapter outputs
    v
stock mdBook HTML renderer
```

The phases are separate because mdBook's built-in links preprocessor scans
chapter text for helper syntax. `render` runs after `index`, so it sees the
final logical chapter arrangement while the original `source_path` is still
available. It replaces eligible chapter content before helper expansion, so
encoded generated values and source text cannot be mistaken for mdBook helper
directives. `rewrite-links` runs after `links`, so ordinary Markdown links
introduced by includes are covered too.

The pipeline delegates through focused contracts:

- the [structured core](structured-core.md) parses source text and exposes the
  parser-neutral document model;
- [route projection](routes.md) assigns deterministic structured chapter
  paths and rejects duplicate final chapter routes before mutation;
- [chapter-link rewriting](link-rewriting.md) resolves or diagnoses authored
  destinations against the projected chapter map; and
- [HTML presentation](html-presentation.md) defines the raw-HTML framing,
  semantic markup, styling hooks, and progressive behavior.

The executable is intentionally thin. It loads configuration, invokes the
core library, projects errors into protocol-friendly diagnostics, and writes
the resulting book. The same core library may later be used by an in-process
build driver without changing its interface, but an in-process integration is
not a v1 requirement.

The exact subprocess protocol, phase registration, renderer support, and
installer commands belong to [Configuration and operations](operations.md).
