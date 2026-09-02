# Selective Structured-Chapter Contract

**Status:** Approved in conversation; written-spec review pending
**Date:** 2026-09-02
**Repository:** `OptimalCNC/mdbook-structured`

## Purpose

Issue #1 is a responsibility correction. `mdbook-structured` must not behave
as a validator or route planner for an entire mdBook. Its build-time semantic
role is limited to transforming registered structured chapters and rewriting
authored Markdown references that point to those chapters.

This record is the governing contract for the `render` and `rewrite-links`
phases. The structured core library and the separately invoked `install`
command retain their own contracts.

## Governing principle

The received `Book` is a transport container, not the module's semantic input.
The module has two admission gates:

```text
Chapter.source_path
    -> RegisteredStructuredSource?
    -> StructuredRoute

Markdown event + current link base
    -> MatchedStructuredReference?
    -> Link edit
```

Only values admitted by one of these gates are semantically observed,
validated, handled, diagnosed, or covered by this module. A value rejected by
a gate is opaque noise. It does not enter a domain type or a later processing
stage, and this contract makes no behavioral promise about it.

The implementation may mechanically traverse the received `Book` or tokenize
Markdown to find admission candidates. That extraction is not semantic
handling. Non-admitted candidates are discarded immediately.

## Registration gate

A chapter is a registered structured chapter only when it is present in the
received `Book` and its `Chapter.source_path` has the exact lowercase extension
`.json`, `.yaml`, or `.yml`.

Registration is based on mdBook's chapter metadata. The plugin does not
discover files from the source tree. The extension discriminator is the only
observation made about a chapter that has not been admitted.

## `render` contract

Only a registered structured chapter enters route projection, structured
parsing, structured rendering, and structured diagnostics.

For an admitted chapter, `render`:

1. parses and renders its structured content; and
2. projects the mdBook-supplied logical path to an extension-preserving `.md`
   route shim.

Examples:

```text
config/runtime.yaml -> config/runtime.yaml.md -> config/runtime.yaml.html
config/README.yaml   -> config/index.yaml.md   -> config/index.yaml.html
```

The route shim is a chapter-path integration mechanism for mdBook's stock
HTML renderer. It is not link rewriting. The source identity remains in
`Chapter.source_path`.

The operation may mutate only the admitted chapter's structured content and
`Chapter.path`. It does not route-plan, parse, or diagnose a non-admitted
chapter. A registered chapter is not silently downgraded to opaque input when
its structured content or required route metadata cannot be processed; such a
failure is scoped to that registered target.

## `rewrite-links` contract

`rewrite-links` constructs a target index only from registered structured
chapters:

```text
normalized structured source_path -> generated HTML route
```

No ordinary chapter route or synthesized convenience alias is part of this
index. Direct structured source identities remain distinct, including
`README.yaml` and `README.json`.

A source chapter need not be structured. It is used only as the authored-byte
and relative-path context while a possible structured reference is extracted.
The Markdown parser is a lexical extractor: only a parser-confirmed authored
destination that resolves to a member of the structured target index becomes a
`MatchedStructuredReference`.

For an admitted reference, the rewriter replaces only the authored path
portion with the matched target's generated HTML route. Query, fragment,
title, surrounding bytes, and authored percent/entity spelling remain part of
the reference contract. Lookup may use lexical dot-segment normalization, but
never filesystem canonicalization or source-containment validation.

The rewriter emits the final structured HTML route directly. mdBook remains
responsible for writing and rendering the resulting chapter; the plugin does
not rely on mdBook's generic `.md` link conversion for structured references.

Anything that does not pass the reference admission gate is outside the
rewriter's semantic domain. It is not resolved, classified, validated,
rewritten, diagnosed, or covered as a plugin behavior. If a source chapter
cannot provide a usable relative base, it contributes no admitted references.

## Diagnostic scope

A book-object diagnostic may be produced only after admission to one of the
module's semantic domains:

- a registered structured chapter or its generated route; or
- a matched reference to a registered structured chapter.

This includes structured parse, resource-limit, lossless-projection, route,
duplicate-identity, and plugin-generated-output integrity failures. Any output
integrity check must be limited to the generated structured target; it must
not become a whole-book lint or inspect unrelated chapter topology.

Objects that never pass an admission gate cannot produce a plugin diagnostic.
Subprocess framing and protocol-decoding failures remain failures of the
external tool interface, not diagnostics about unrelated `Book` objects.

## Ownership and exclusions

Everything outside the admission gates is outside `mdbook-structured`'s
semantic awareness. mdBook and the author remain responsible for book
structure, source loading and containment, ordinary chapter routes, static
files, final HTML rendering, and ordinary links. This contract does not make
the plugin a general path validator, filesystem auditor, ordinary-route map,
or whole-book linter.

## Verification obligations

Tests establish the admission gates and the positive transformations promised
above. A fixture containing unrelated objects may be used as a sentinel for
accidental scope expansion, but it does not define behavior for those objects
or create an exhaustive test obligation for them.

The governing acceptance properties are:

1. only registered structured chapters enter structured rendering and route
   projection;
2. only matched structured references enter link rewriting;
3. generated structured routes and direct structured source links agree,
   including distinct README routes; and
4. no unrelated chapter, path, link, file, or value becomes a plugin semantic
   input or diagnostic source.

Until implementation and normative documentation are aligned with this
record, this record takes precedence over descriptions of broader whole-book
preflight behavior.
