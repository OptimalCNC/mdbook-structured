# mdbook-structured Design

Status: Approved design for v1
Date: 2026-08-28

## 1. Purpose and Scope

mdbook-structured is an external Rust mdBook preprocessor backed by a
reusable format-neutral library. It renders selected JSON and YAML source
files as readable, styled pages in mdBook's stock HTML output. The source
files remain the authoritative files on disk, and the generated page remains
part of the ordinary mdBook book: navigation, themes, search, edit links, and
the stock renderer continue to own their usual responsibilities.

The architecture is intentionally general, but the v1 input set is fixed to
.json, .yaml, and .yml. A file is eligible only when it has already been
loaded by mdBook as a chapter listed in SUMMARY.md. The implementation does
not scan the source tree or discover unlisted files.

V1 includes:

* JSON and a documented JSON-compatible subset of YAML;
* a format-neutral StructuredDocument model with source provenance;
* build-time semantic HTML rendering and progressive JavaScript enhancement;
* book-wide Markdown link rewriting for registered structured chapters;
* output-path collision preflight and fail-fast diagnostics;
* an installer that generates starter CSS and JavaScript without editing
  book.toml; and
* tests that verify semantic behavior at the parser, renderer, preprocessor,
  and installer seams.

V1 does not include a custom mdBook backend, a fork of mdBook, JSON/YAML
parsers in mdBook core, implicit file discovery, non-HTML renderer support,
schema-aware or domain-specific views, generated JSON Pointer anchors,
persistent expansion state, print-specific styling, or automatic
configuration-file edits. Full YAML features are also outside the v1 policy;
the accepted subset and its rejection rules are defined in Section 4.

The ownership split is:

* mdBook core owns book structure, processing order, chapter paths, URL
  generation, and rendering infrastructure;
* mdbook-structured-core owns source-format interpretation, normalization,
  diagnostics, and structured-page rendering; and
* mdbook-structured owns the mdBook subprocess protocol, chapter selection,
  book-wide link mapping, and the install command.

This keeps the external seam small while allowing future format adapters and
renderers to evolve without coupling their parser types to mdBook.

## 2. Authoring Contract

An author may list a structured source directly in SUMMARY.md, alongside
ordinary Markdown chapters:

~~~md
# Summary

- [Introduction](README.md)
- [Runtime configuration](config/runtime.yaml)
- [Protocol manifest](schemas/manifest.json)
- [Deployment guide](deployment.md)
~~~

mdBook loads each listed target as UTF-8 chapter text. The structured
preprocessor dispatches from Chapter.source_path, which is the real source
path, and leaves that field unchanged. The title supplied in SUMMARY.md
continues to be the chapter title. The logical chapter path and output path
continue to follow mdBook's normal rules.

Only listed chapters are transformed. A Markdown link to an unlisted JSON or
YAML file does not create a page, and a source file that is merely present in
the source directory is ignored. A link to a listed structured chapter is
rewritten by the separate link phase described in Section 5.

Authors register the two preprocessor phases and the generated assets in
book.toml. The installer prints the required entries but never inserts them.
When an additional-css or additional-js array already exists, authors append
the printed path to that array rather than replacing its existing entries.

The authoring contract is deliberately aligned with mdBook. Relative paths,
README/index handling, query strings, fragments, and output names are not
redefined by this tool. A structured source with the same logical stem as
another chapter may collide after mdBook changes both extensions to .html;
such collisions are diagnosed before rendering rather than resolved
implicitly.

## 3. Processing Architecture

The stock mdBook pipeline is extended with two registrations of the same
executable:

~~~text
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
    |  parse eligible sources, validate, render semantic HTML
    v
mdBook's links preprocessor
    |  expand the normal Markdown helpers/includes
    v
mdbook-structured rewrite-links
    |  resolve Markdown links to registered chapter outputs
    v
stock mdBook HTML renderer
~~~

The recommended registration is:

~~~toml
[preprocessor.structured]
command = "mdbook-structured render"
after = ["index"]
before = ["links"]
renderers = ["html"]

[preprocessor.structured-links]
command = "mdbook-structured rewrite-links"
after = ["links"]
renderers = ["html"]
~~~

The phases are separate because mdBook's built-in links preprocessor scans
chapter text for helper syntax. render runs after index, so dispatch sees
the final logical chapter arrangement while the original source_path is
still available. It replaces eligible chapter content before helper
expansion; escaped generated values and source text therefore cannot be
mistaken for raw mdBook helper directives. The HTML renderer must entity-escape
helper-token delimiters in generated text and attributes as well as applying
ordinary HTML escaping; browser-visible text remains exact while mdBook's
helper scanner cannot recognize it. rewrite-links runs after links, so
ordinary Markdown links introduced by includes are covered too.

Both commands implement the standard external-preprocessor protocol. They
read the complete Book JSON and preprocessor context from stdin, write one
transformed Book JSON value to stdout, and write human diagnostics only to
stderr. They accept only the html renderer. The executable rejects a
malformed protocol value or an unsupported renderer instead of silently
passing through a partially processed book.

Before mutating any chapter, render performs a book-wide output-path
preflight. The preflight uses each chapter's logical path and mdBook's
with_extension("html") rule. Any duplicate destination is an error naming
all conflicting source chapters. No automatic renaming is attempted.

The executable is intentionally thin. It loads configuration, invokes the
core library, projects errors into protocol-friendly diagnostics, and writes
the resulting book. The core library can later be used by an in-process build
driver without changing its interface, but an in-process integration is not a
v1 requirement.

## 4. Structured Core

mdbook-structured-core is the deep module at the format seam. Its callers
need to provide a format, UTF-8 source text, a source name, and limits; they
receive a StructuredDocument or a structured diagnostic. Parser-library
types do not cross this interface.

The conceptual public model is:

~~~rust
struct StructuredDocument {
    format: StructuredFormat,
    original_source: String,
    root: Node,
    stats: DocumentStats,
}

struct Node {
    span: SourceSpan,
    value: NodeValue,
}

enum NodeValue {
    Mapping(Vec<MappingEntry>),
    Sequence(Vec<Node>),
    String(String),
    Number(NumberLexeme),
    Boolean(bool),
    Null,
}

struct MappingEntry {
    decoded_key: String,
    key_span: SourceSpan,
    value: Node,
}
~~~

SourceSpan retains byte offsets and line/column information. Sequence
positions are represented by their ordered position, and mapping entries
retain insertion order. Decoded keys are unique strings. Numbers retain their
validated lexical form rather than being converted through f64. Strings
retain their decoded value exactly: the core performs no trimming, case
normalization, or simplification. The original source is retained separately
so comments, quoting, whitespace, line endings, and formatting remain
available to the raw-source view.

Internally, format adapters satisfy a small parser seam equivalent to:

~~~rust
trait FormatAdapter {
    fn parse(
        &self,
        source: &str,
        source_name: &Path,
        limits: &Limits,
    ) -> Result<StructuredDocument, Diagnostic>;
}
~~~

The JSON adapter uses json-syntax (0.12.x) for syntax, lexical, and span
information. Duplicate object names are rejected so the format-neutral
mapping invariant remains unambiguous. The YAML adapter uses
saphyr-parser (0.0.12) and applies the v1 policy below before constructing
the public model.

V1 YAML policy is strict single-document YAML 1.2 Core Schema with a
JSON-compatible representation. The adapter rejects duplicate keys,
non-string keys, explicit tags, anchors, aliases, merge keys, multiple
documents, non-finite numbers, malformed syntax, and all configured limit
violations. It never constructs arbitrary objects from YAML tags and never
recursively expands aliases. Values such as yes, no, on, off, dates, and
timestamps remain strings unless the Core Schema rules classify them as a
supported scalar. This policy is explicit so that the tool does not pretend
that general YAML is JSON.

The parser and renderer enforce bounded input, node, and depth limits. The v1
defaults are:

~~~toml
max-input-bytes = 1048576
max-nodes = 10000
max-depth = 64
~~~

The large-container threshold used for initial HTML expansion is 100
immediate children. Limits cause a diagnostic before unbounded expansion or
output generation. A diagnostic identifies the source name, category, line
and column when available, and the structured key path when the parser can
establish one.

Future TOML, XML, CSV, schema, or project-specific adapters may target the
same model. They must preserve the model's ordering, scalar, provenance, and
error invariants without exposing their parser-specific representations.

## 5. Paths and Links

### Output paths and collisions

The tool does not invent a routing scheme. For every chapter it uses the
logical Chapter.path already established by mdBook and applies the stock
HTML renderer's rule:

~~~text
Chapter.path.with_extension("html")
~~~

The real source remains in Chapter.source_path, so edit links and source
diagnostics continue to point to config/runtime.yaml or
schemas/manifest.json. A Markdown chapter and a structured chapter with the
same logical stem, or two structured files with different extensions, can
therefore map to one HTML destination. This is a normal repository shape for
configuration and schema variants, not a case to hide. The preflight fails
with every conflicting chapter listed and never chooses a winner or renames a
page.

### Chapter-aware Markdown links

rewrite-links builds a map from both the source paths and logical chapter paths
in the received Book to their mdBook output paths. It resolves a relative
Markdown destination from the current chapter using mdBook's path conventions,
then rewrites it only when the normalized destination identifies a registered
chapter. For example:

~~~md
[Runtime configuration](config/runtime.yaml)
~~~

becomes a link to the corresponding config/runtime.html page, with the
relative URL calculated from the current chapter in the same manner as
mdBook. README/index behavior is inherited from the chapter map rather than
reimplemented as a special case.

The rewriter edits Markdown link spans using mdBook's Markdown parser. It does
not reserialize an entire chapter. It preserves query strings, percent
encoding, and fragment text verbatim. Fragments are opaque: the tool does not
parse JSON Pointer syntax and does not generate or validate pointer anchors in
v1.

The following are left unchanged:

* images and image destinations;
* raw HTML links, code blocks, and ordinary scalar text;
* external, protocol-relative, and other non-local URI schemes;
* fragment-only links;
* destinations that do not identify a registered chapter; and
* destinations that already end in .html.

Reference-style links are supported by rewriting their definition once. An
image-only reference definition is unchanged. If one definition is shared by
an image and a normal link and rewriting it would change the image, the
preprocessor fails with a clear diagnostic rather than guessing. Strings
inside JSON/YAML values are never scanned as Markdown and are never rewritten.

Authored fragments remain available for future or user-provided anchors, but
the structured renderer does not create JSON Pointer-based anchors in v1.

## 6. HTML Presentation

The preprocessor emits complete semantic HTML at build time. The output is
readable with JavaScript disabled; JavaScript is an enhancement layer and
never parses the source or owns the data model. The visual reference is
Firefox's JSON Viewer: compact monospaced rows, indentation, twisties,
key/value alignment, restrained hover states, and distinct scalar coloring.
Those interaction and styling ideas are independently implemented so the page
remains part of the mdBook theme.

The root value is rendered directly; no synthetic root row is added.
Mappings and sequences are container rows using native HTML details and
summary elements. The first two nesting levels are open initially. A
container with more than 100 immediate children starts closed, even when it
is shallow. The generated page includes controls to expand or collapse all
containers on the active page. They do not affect other pages and do not
persist state between visits.

Keys and string values are decoded and displayed without surrounding quotes.
CSS type markers distinguish strings, numbers, booleans, and null, so a string
true remains visibly different from boolean true. Empty keys and empty
strings receive an explicit visual marker. Long strings retain their complete
content and source whitespace; CSS handles readable wrapping rather than
truncating or simplifying values. Mapping order and sequence order are
preserved.

A collapsed Original source section contains the exact original UTF-8 text,
unchanged, in a format-labelled code block. It supports copy/paste,
syntax-oriented review, and comparison when the structured view is not the
right representation. It is excluded from expand/collapse-all. Every value
inserted into HTML is escaped, including helper-token delimiters such as
double opening braces; source content is never interpreted as markup. The
browser still displays the original characters.

The supplied CSS and JavaScript expose a small semantic hook vocabulary for
the document root, node kinds, scalar types, and containers. Those hooks are
the stable contract used by the enhancement script and tests. Incidental
wrapper nesting, attribute order, and additional styling classes remain
changeable, and authors may override the starter assets through mdBook's
normal theme configuration. Print-specific behavior is not a v1 requirement.

## 7. Configuration and Operations

The v1 executable exposes render, rewrite-links, and install. The first two
commands use the standard mdBook preprocessor protocol: complete Book JSON
enters on stdin, transformed Book JSON leaves on stdout, and diagnostics go
only to stderr. Both commands are restricted to the stock html renderer and
fail on protocol, configuration, parsing, validation, rendering, or
link-rewrite errors.

The recommended complete configuration is:

~~~toml
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
~~~

render converts eligible JSON/YAML chapters before helper expansion.
rewrite-links runs afterward and covers links in ordinary Markdown, including
links introduced by includes. Files not present as chapters are never
discovered implicitly. Each registered structured chapter must parse and
render successfully. Malformed input, unsupported YAML constructs, duplicate
keys, and resource-limit violations stop the build with a filename, source
location, and, when available, structured path. There is no silent fallback
that leaves a broken registered source as raw text.

The fixed v1 extension set is .json, .yaml, and .yml; unrelated chapters pass
through unchanged. The limits are explicit configuration, not silent
truncation. Unknown or invalid configuration is an error.

mdbook-structured install [DIR] writes mdbook-structured.css and
mdbook-structured.js under DIR, which defaults to the current book root. It
never reads or modifies book.toml. It prints instructions showing where
authors should append the asset paths in additional-css and additional-js;
registration remains an author decision.

The installer processes the two files sequentially. It creates an absent file,
leaves a byte-identical existing file unchanged, and stops on the first
different existing file or unexpected write error. Partial side effects are
allowed; rollback and atomic multi-file updates are intentionally out of
scope. The generated assets are starter implementations using mdBook theme
variables and can be edited or replaced by the author.

## 8. Verification and Evolution

Verification targets consumer-visible behavior, not the incidental
serialization of HTML or JSON. This follows mature artifact-producing test
patterns: mdBook decodes generated search data and checks selected fields,
its protocol tests deserialize and round-trip typed book values, its browser
tests assert selectors, text, counts, and attributes, and the parser crates
use table-driven/specification corpora with focused regression properties.
Test inputs are small, hand-authored source fixtures supplemented by bounded
generated valid documents. Expected behavior is expressed as typed assertions
and invariants. V1 does not use whole-file HTML snapshots, generated-book
byte comparisons, or approval files for ordinary renderer changes.

The core test surface is the public StructuredDocument interface. Table-driven
positive fixtures cover representative JSON and accepted YAML: mappings and
sequences, nesting, insertion order, decoded keys and strings, empty values,
booleans versus strings, lexical numbers, source spans, and exact
original-source retention. A property-based generator produces bounded
JSON-compatible trees and serializes them into valid JSON. YAML positive
coverage uses curated accepted fixtures and the parser's conformance corpus;
the core does not add a YAML serializer solely for tests. The resulting
documents must parse, retain their structural statistics, and satisfy node,
order, and type invariants. For JSON, an independent semantic projection
through serde_json::Value may
corroborate basic value meaning; it does not replace span or lexical-form
assertions. Parser-library conformance suites remain the authority for general
grammar coverage.

The renderer is tested through a standards-compliant HTML parser, never by
comparing serialized HTML bytes. A small test-only semantic probe walks the
.structured-document subtree and records an ObservedDocument: node kind,
mapping key or sequence index, scalar text and type, child count,
container-open state, and raw-source text. Tests assert this observation
against concise expected values. They verify that source nodes appear in
order, scalar types remain distinguishable, text is escaped and complete, the
root is rendered directly, the two-level and large-container rules are
applied, and the original source is unchanged. The probe uses only the
documented semantic hooks; wrapper nesting, whitespace between tags, attribute
order, and ordinary CSS classes remain free to change. Expected observations
are written independently of renderer internals, so the probe cannot merely
reproduce the renderer's output. One positive fixture contains literal mdBook
helper-looking text in both a scalar and the original source; the observed
browser text must retain it without invoking a helper.

JavaScript is tested at the behavior seam. One browser-level smoke suite,
following mdBook's selector-oriented browser tests, loads a built fixture and
asserts visible node counts and text, initial disclosure state, expand-all and
collapse-all behavior, active-page scoping, and the absence of persistent
state after a fresh load. No screenshot or pixel baseline is required. CSS
receives semantic-hook coverage and a maintained manual visual check; visual
styling is intentionally author-overridable.

The preprocessor is tested first with in-memory Book values. These tests
assert typed protocol results: only eligible chapters change, source_path is
preserved, unrelated chapters and metadata are untouched, output collisions
are reported, and rewritten destinations follow the chapter map. Link
rewriting is tested against the Markdown AST and spans, including reference
definitions. One small mdBook build fixture verifies the actual ordering with
index, links, includes, and the stock HTML renderer. That fixture is inspected
through parsed output paths, DOM links, and semantic probes, not expected HTML
files. The fixture includes a literal helper-looking value to verify the
render-before-links protection. A single end-to-end fixture proves wiring; it
does not duplicate every core or renderer case.

Only high-value failure tests are required. They cover a malformed registered
source with location-bearing diagnostics, one representative of each security
or policy rejection class that could otherwise alter meaning (for example
duplicate keys and forbidden aliases or tags), each resource-limit boundary,
an output-path collision, and HTML-injection escaping. They do not duplicate
every malformed syntax variant already covered by the parser libraries.
Assertions check error category, source path, location, and structured path
rather than brittle prose. A new negative case is added when a regression or
security requirement makes it valuable.

Installer tests observe filesystem state transitions: creation, byte-identical
reruns, refusal to overwrite differing files, sequential stopping, printed
append instructions, and unchanged book.toml. They compare hashes or selected
required markers only; the complete generated asset text is not a test oracle.

Every discovered bug adds the smallest regression at the seam where it is
observable. Parser upgrades rerun the positive corpus and properties. Future
format adapters and renderer layers must satisfy the same semantic contract;
they do not add parser-specific assertions to the mdBook protocol. JSON
Pointer-generated anchors, persistent state, alternate backends, and deeper
mdBook integration remain deferred. Any proposed mdBook core change remains a
separate generic contribution: chapter-aware link resolution, duplicate
output-path diagnostics, or an explicit UTF-8 textual-source contract. A
dependency-reporting API is considered only after a prototype establishes a
reproducible mdbook serve rebuild problem. No first-class content-kind field or
format-specific behavior enters mdBook core without a versioned protocol
design.

The v1 acceptance criterion is semantic: representative and generated valid
documents render completely and in order, the stock mdBook build exposes the
expected links and source provenance, the small set of critical invalid cases
fails loudly, and style or markup refactoring does not require rewriting large
snapshots.
