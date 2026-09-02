# Structured-Target Link Rewriting

`rewrite-links` constructs one target index from registered structured
chapters:

```text
normalized structured source_path -> generated HTML route
```

Only direct structured source identities enter the index. This keeps
`README.yaml` and `README.json` distinct, mapping them to `index.yaml.html` and
`index.json.html`, respectively.

To find possible matches, the rewriter uses the Markdown parser to extract
authored link-destination spans and uses the current chapter path as their
relative base. Extraction alone does not admit a link destination. A link
destination becomes a matched structured reference only when lexical relative
resolution identifies a member of the structured target index. If a source
chapter cannot supply a usable relative base, it contributes no matched
structured reference.

For example:

```md
[Runtime configuration](config/runtime.yaml)
```

is rewritten to the relative URL for `config/runtime.yaml.html` when that
source identity belongs to a registered structured chapter.

## Matched-reference edits

For a matched structured reference, the rewriter replaces only the authored
path portion with the target's generated HTML route. Query, fragment, title,
surrounding Markdown bytes, and authored percent or entity spelling remain
part of the edit contract. Fragments are opaque: the preprocessor neither
interprets JSON Pointer syntax nor creates or validates pointer anchors.

Target matching may normalize lexical dot segments. It does not canonicalize
the filesystem or validate source containment. The generated HTML route is
emitted directly, so this operation does not rely on mdBook's generic `.md`
link conversion. This is separate from the `.md` chapter-path shim applied by
[`render`](routes.md).

The rewriter changes only the authored link-destination bytes and does not
reserialize a chapter. If extraction does not produce a matched structured
reference, no semantic input exists for this operation: the candidate is not
resolved further, classified, validated, rewritten, or diagnosed by this
module.

A reference diagnostic may occur only after a link destination has become a
matched structured reference, and it is scoped to producing the promised edit
for that reference. The rewriter does not construct an ordinary chapter route
map or diagnose book topology outside the structured target index.
