# HTML Presentation

The preprocessor emits complete semantic HTML at build time. JavaScript is an
enhancement layer: it never parses the source or owns the data model. The
output remains readable when JavaScript is disabled.

## Renderer boundary

Generated chapter content is framed as one contiguous raw HTML block with no
blank line that could terminate the block in mdBook's Markdown parser.
Source-derived line breaks are entity-encoded where necessary rather than
allowed to split that block. This framing ensures that generated elements and
data cannot be reinterpreted as Markdown before the stock HTML renderer sees
them.

The structured-page renderer uses context-specific, one-pass encoding that
both HTML-escapes data and neutralizes mdBook helper-token delimiters in
generated text and attributes. Ordinary displayable browser text retains its
value while mdBook's helper scanner cannot recognize it.

Each page starts with a visible `h1` whose text comes from `Chapter.name`,
followed by the structured data tree. The heading is chapter framing, not a
node in the data model.

## Structured tree

The visual reference is Firefox's JSON Viewer: compact monospaced rows,
indentation, twisties, key/value alignment, restrained hover states, and
distinct scalar coloring. Those interaction and styling ideas are
independently implemented so the page remains part of the mdBook theme.

The data root is rendered directly as an always-visible node; no synthetic
data-root row, label, or disclosure is added. Nested mappings and sequences are
container rows using native HTML `details` and `summary` elements. Containers
at model depth two start open, while deeper containers start closed. A nested
container with more than 100 immediate children starts closed, even at depth
two.

Sequence summaries include their item count. A mapping inside a sequence shows
its first two immediate scalar fields in source order as a compact preview,
with its positional index shown as muted metadata at the right. An ellipsis
indicates additional fields. Items without scalar fields show their field or
item count instead. Preview keys and values may be clipped visually to keep
rows compact; expanding an item reveals every field with its full value.
Previews are generated at build time and remain available without JavaScript.

At every model depth, each non-empty mapping or sequence group is classified
from its immediate rendered model/DOM children, including children inside a
closed disclosure. A mixed group has at least one foldable mapping or sequence
child, so every row reserves a disclosure gutter before its label: foldable rows
place their native marker there and scalar rows leave it empty. An all-scalar
group has only string, number, boolean, or null children; it omits only that
marker-specific gutter and any root marker clearance while preserving ordinary
structural nesting indentation and one sibling label column. A nested
container's summary remains a foldable row in its parent group, while its own
children are classified separately. Empty containers have no child label group,
so their existing disclosure behavior is preserved. The alignment anchor is the
start of the row's content: a decoded mapping key, a scalar item's sequence
index, or a container item's preview. It is not the first visible glyph or raw
source indentation or quoting. The heading,
action buttons, and Original source disclosure are outside this data-row
alignment system.

The generated page includes controls to expand or collapse all containers on
the active page. They do not affect other pages and do not persist state
between visits.

Keys and string values are decoded and displayed without surrounding quotes.
CSS type markers distinguish strings, numbers, booleans, and null, so a string
`true` remains visibly different from boolean `true`. Empty keys and empty
strings receive an explicit visual marker. Long strings retain their complete
content and source whitespace; CSS handles readable wrapping rather than
truncating or simplifying values. Each authored line-break sequence in a
rendered string value or mapping key has a muted `↵` marker immediately before
the preserved break, so it remains distinguishable from responsive wrapping.
The marker is absent from copied text, accessibility text, and Original source.
Mapping and sequence order are preserved.

## Original source

A collapsed Original source section contains
[`loaded_source`](structured-core.md#public-model) without tool-level trimming,
normalization, or simplification, in a format-labelled code block. It supports
copy/paste, syntax-oriented review, and comparison when the structured view is
not the right representation. It is excluded from expand/collapse-all.

Every value inserted into HTML is encoded for its HTML context in one pass;
source content is never interpreted as markup or Markdown. For ordinary
displayable text, this encoding changes markup bytes without changing the
browser-visible characters. V1 defines no special presentation or
browser-text fidelity contract for non-displayable control characters. The
model and `loaded_source` still retain the parser and mdBook inputs without
tool-level stripping.

## CSS and JavaScript

The supplied CSS and JavaScript expose a small semantic hook vocabulary for
the document root, node kinds, scalar types, and containers. Those hooks are
the stable contract used by the enhancement script and tests. Incidental
wrapper nesting, attribute order, and additional styling classes remain
changeable.

Authors register or override the starter assets through mdBook's normal theme
configuration, as described in [Configuration and operations](operations.md).
Print-specific behavior is not a v1 requirement.
