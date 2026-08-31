# Verification

Verification targets consumer-visible behavior rather than the incidental
serialization of HTML or JSON. This follows mature artifact-producing test
patterns: mdBook decodes generated search data and checks selected fields, its
protocol tests deserialize and round-trip typed book values, its browser tests
assert selectors, text, counts, and attributes, and parser crates use
table-driven or specification corpora with focused regression properties.

Test inputs are small, hand-authored source fixtures supplemented by bounded
generated valid documents. Expected behavior is expressed as typed assertions
and invariants. V1 does not use whole-file HTML snapshots, generated-book byte
comparisons, or approval files for ordinary renderer changes.

## Structured core

The core test surface is the public
[`StructuredDocument`](structured-core.md#public-model) interface.
Table-driven positive fixtures cover representative JSON and YAML documents
accepted by the selected parser libraries: mappings and sequences, nesting,
insertion order, decoded keys and strings, empty values, booleans versus
strings, lexical numbers, source spans, and exact `loaded_source` retention.
YAML scalar-kind expectations come from `rlsp-yaml-parser` 0.11.1's Core
schema rather than a project-owned classification table.

A property-based generator produces bounded JSON-compatible trees and
serializes them into valid JSON. YAML positive coverage uses small curated
parser-accepted fixtures; the core does not add a YAML serializer or duplicate
the parser library's general grammar suite solely for tests. Successful
fixtures must project losslessly, retain their structural statistics, and
satisfy node, order, type, and normalized-coordinate invariants, including
with multibyte UTF-8 before a span.

For JSON, an independent semantic projection through `serde_json::Value` may
corroborate basic value meaning; it does not replace span or lexical-form
assertions. Parser-library conformance suites remain the authority for general
grammar coverage. A change to the exact YAML parser pin must pass the same
public core contract tests before adoption.

## HTML and JavaScript

The renderer is tested through mdBook's Markdown parser followed by a
standards-compliant HTML parser, never by comparing serialized HTML bytes. A
small test-only semantic probe walks the `.structured-document` subtree and
records an `ObservedDocument`: heading text, node kind, mapping key or sequence
index, scalar text and type, child count, container-open state, hard-break
marker placement, and raw-source text.

Tests assert this observation against concise expected values. They verify
that `Chapter.name` becomes the visible `h1`, source nodes appear in order,
scalar types remain distinguishable, ordinary displayable text is escaped and
complete, the data root is rendered directly without a synthetic disclosure,
authored breaks remain distinct from responsive wrapping without changing
selected text, the nested-container disclosure rules are applied, and
`loaded_source` is retained without tool-level modification. Passing generated
content through Markdown also verifies the [raw HTML
framing](html-presentation.md#renderer-boundary).

The probe uses only documented semantic hooks; wrapper nesting, whitespace
between tags, attribute order, and ordinary CSS classes remain free to change.
Expected observations are written independently of renderer internals, so the
probe cannot merely reproduce the renderer's output. One positive fixture
contains literal mdBook helper-looking text in both a scalar and
`loaded_source`, plus HTML-looking text in a scalar. The observed browser text
must retain both without invoking a helper or creating injected elements. V1
has no control-character-specific presentation acceptance test.

JavaScript is tested at the behavior seam. One browser-level smoke suite,
following mdBook's selector-oriented browser tests, loads a built fixture and
asserts visible node counts and text, initial disclosure state, expand-all and
collapse-all behavior, active-page scoping, and the absence of persistent
state after a fresh load. It also checks that scalar and foldable sibling rows
share their rendered label column and that a native root marker does not escape
the structured-document boundary. Geometry assertions cover the all-scalar
direct root, the nested all-scalar `display` mapping, and the nested all-scalar
`ports` sequence after expansion, as well as mixed-root and mixed nested
alignment. They use the start of each decoded rendered label as the anchor and
verify that all-scalar groups omit only marker clearance while retaining
structural indentation. No screenshot or pixel baseline is required. CSS
receives semantic-hook coverage and a maintained manual visual check; visual
styling is intentionally author-overridable.

## Preprocessor and mdBook integration

The preprocessor is tested first with in-memory `Book` values. These tests
assert typed protocol results: only eligible chapters change, `source_path` is
preserved, structured logical paths gain the source-extension shim, unrelated
chapters and metadata are untouched, projected chapter-route collisions and
exact static-source conflicts are reported, and rewritten destinations follow
the chapter map. Two structured README chapters with different source
extensions but the same post-index logical path must project to distinct
outputs. Direct source links to both must rewrite independently, and unused
shared convenience aliases must not fail preprocessing. A separate case adds
an actual `index.md` chapter at a shared alias destination and verifies that
the exact chapter wins without ambiguity. Link rewriting is tested against the
Markdown AST and spans, including reference definitions. Capability tests
exercise `render supports html` and `rewrite-links supports html`.

One small mdBook build fixture verifies the actual ordering with `index`,
`links`, includes, and the stock HTML renderer. It is inspected through parsed
output paths, DOM links, and semantic probes rather than expected HTML files.
It verifies `runtime.yaml.html` plus distinct `index.yaml.html` and
`index.json.html` routes, direct source-path links to both README pages, a
unique README/index/directory alias, and stock publication of the raw
`runtime.yaml` file. The fixture includes a literal helper-looking value to
verify render-before-links protection. A single end-to-end fixture proves
wiring; it does not duplicate every core or renderer case.

## High-value failures

Negative tests are added only when they protect a critical semantic or
security boundary. V1 covers a malformed registered source with
location-bearing diagnostics, duplicate decoded keys, a parsed construct that
cannot be projected losslessly, each resource-limit boundary, an exact
projected-route collision, an exact static-source conflict, and a projected
route or rewritten destination that stock mdBook would corrupt because it
contains a literal `.md`. It also covers an authored convenience alias with
multiple chapter candidates, which must fail without rejecting the same book
when that alias is unused.

These tests do not duplicate every malformed syntax variant already covered
by parser libraries or define an independent YAML behavior matrix. Assertions
check error category, source path, location, and structured path rather than
brittle prose. A new negative case is added when a regression, data-loss risk,
or security requirement makes it valuable.

## Installer

Installer tests observe filesystem state transitions: creation,
byte-identical reruns, refusal to overwrite differing files, sequential
stopping, printed add-path instructions, and unchanged `book.toml`. They
compare hashes or selected required markers only; the complete generated asset
text is not a test oracle.

## Acceptance criterion

Representative and generated valid documents render completely and in order;
the stock mdBook build exposes the expected links and source provenance; the
small set of critical invalid cases fails loudly; and style or markup
refactoring does not require rewriting large snapshots.
