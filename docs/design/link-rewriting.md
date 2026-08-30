# Chapter Link Rewriting

`rewrite-links` builds exact chapter lookups and convenience-alias candidate
sets from the source paths and logical chapter paths in the received `Book`.
Every chapter source path is exact. Independently authored logical paths are
also exact, while paths synthesized from a README source by mdBook's `index`
phase are convenience aliases. Both kinds of lookup resolve to the outputs
established by [route projection](routes.md).

The rewriter resolves a relative Markdown destination from the current chapter
using mdBook's path conventions, then consults the exact lookup before any
convenience aliases.

For example:

```md
[Runtime configuration](config/runtime.yaml)
```

becomes a link to `config/runtime.yaml.html`, with the relative URL calculated
from the current chapter in the same manner as mdBook.

## README and index aliases

For a structured source whose file stem is `README`, the map also registers
mdBook's post-index `index.md` path and its parent-directory form as aliases
for the projected `index.<source-extension>.html` page. The source path itself,
such as `README.yaml`, remains an exact chapter lookup even though `render` has
already replaced `Chapter.path` with the shimmed path.

An actual registered chapter at a destination takes precedence over a
convenience alias. Otherwise, an alias with one candidate is rewritten, while
an alias with multiple candidates fails with a diagnostic naming each source
and projected output. Candidate sets retain every README target: the tool does
not reject otherwise valid chapters or choose one by `SUMMARY.md` order.

For example, direct links to `README.yaml` and `README.json` in the same
directory resolve to `index.yaml.html` and `index.json.html`. A link to their
shared `index.md` or directory-style alias is ambiguous, in the absence of an
exact chapter at that destination, and fails only when that link appears in
authored Markdown.

## Rewriting rules

The rewriter edits Markdown link spans using mdBook's Markdown parser. It does
not reserialize an entire chapter. It preserves query strings, percent
encoding, and fragment text verbatim. Fragments are opaque: the tool does not
parse JSON Pointer syntax and does not generate or validate pointer anchors in
v1.

The stock HTML renderer subsequently rewrites any local URL containing a
literal `.md` substring, even when that substring is not the final extension.
The [route preflight](routes.md#preflight) therefore rejects a projected path
such as `guide.md.yaml.html`. `rewrite-links` also fails if a query or fragment
would introduce literal `.md` into an otherwise valid rewritten destination.
These checks prevent silent corruption while leaving already encoded text
untouched. V1 does not invent a URL encoding to work around this mdBook
behavior.

The following are left unchanged:

- images and image destinations;
- raw HTML links, code blocks, and ordinary scalar text;
- external, protocol-relative, and other non-local URI schemes;
- fragment-only links;
- destinations with neither an exact chapter nor an alias candidate; and
- destinations that already end in `.html`.

Reference-style links are supported by rewriting their definition once. An
image-only reference definition is unchanged. If one definition is shared by
an image and a normal link and rewriting it would change the image, the
preprocessor fails with a clear diagnostic rather than guessing. Strings
inside JSON/YAML values are never scanned as Markdown and are never rewritten.

Authored fragments remain available for future or user-provided anchors, but
the structured renderer does not create JSON Pointer-based anchors in v1.
