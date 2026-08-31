# Evolution

Every discovered bug adds the smallest regression at the seam where it is
observable. Parser upgrades rerun the positive corpus and properties. Future
format adapters and renderer layers must satisfy the same semantic contract;
they do not add parser-specific assertions to the mdBook protocol.

## Format extensions

Future adapters for tree-shaped, JSON-compatible formats may target the
existing [`StructuredDocument`](structured-core.md#public-model). They must
preserve its ordering, scalar, provenance, and error invariants without
exposing parser-specific representations. A format whose parsed values cannot
be represented losslessly requires a different model or rendering layer
rather than a lossy adapter.

## Deferred features

JSON Pointer-generated anchors, schema-aware and domain-specific views,
persistent expansion state, alternate backends, print-specific behavior, and
deeper mdBook integration remain deferred. These exclusions preserve the
[v1 boundary](overview.md) until implementation evidence establishes a need
to change it.

## Possible mdBook contributions

Any proposed mdBook core change remains a separate generic contribution.
Candidates include chapter-aware link resolution, duplicate output-path
diagnostics, and an explicit UTF-8 textual-source contract.

A dependency-reporting API is considered only after a prototype establishes a
reproducible `mdbook serve` rebuild problem. No first-class content-kind field
or format-specific behavior enters mdBook core without a versioned protocol
design.
