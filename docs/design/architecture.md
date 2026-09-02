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
    |  admit registered sources, project route shims, render semantic HTML
    v
mdBook's links preprocessor
    |  expand the normal Markdown helpers/includes
    v
mdbook-structured rewrite-links
    |  edit links that match the structured target index
    v
stock mdBook HTML renderer
```

The phases are separate because mdBook's built-in links preprocessor scans
chapter text for helper syntax. `render` runs after `index`, so an admitted
chapter supplies its mdBook-established logical path while retaining its
original `source_path`. The preprocessor replaces that registered chapter's
content before helper expansion, so encoded generated values and source text
cannot be mistaken for mdBook helper directives. `rewrite-links` runs after
`links`, so link destinations introduced by includes become candidates for the
same structured-reference admission gate.

The phases use separate domain inputs. `render` admits chapters from
`Chapter.source_path`; `rewrite-links` builds its target index from those
registered structured sources and admits only parser-confirmed link
destinations that resolve to an indexed target. Traversing the remaining
`Book` or parsing Markdown to find candidates does not make rejected candidates
semantic input.

The pipeline delegates through focused contracts:

- the [structured core](structured-core.md) parses source text and exposes the
  parser-neutral document model;
- [structured route projection](routes.md) assigns deterministic paths to
  registered structured chapters and protects the integrity of generated
  structured routes;
- [structured-target link rewriting](link-rewriting.md) indexes structured targets and
  edits only authored link destinations admitted by that index; and
- [HTML presentation](html-presentation.md) defines the raw-HTML framing,
  semantic markup, styling hooks, and progressive behavior.

The executable is intentionally thin. It loads configuration, invokes the
core library, projects errors into protocol-friendly diagnostics, and writes
the resulting book. The same core library may later be used by an in-process
build driver without changing its interface, but an in-process integration is
not a v1 requirement.

The exact subprocess protocol, phase registration, renderer support, and
installer commands belong to [Configuration and operations](operations.md).
