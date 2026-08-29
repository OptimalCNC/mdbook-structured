# Structured Core

mdbook-structured-core is the deep module at the format seam. Callers provide
a format, UTF-8 source text, a source name, and limits; they receive a
`StructuredDocument` or a structured diagnostic. Parser-library types do not
cross this interface.

## Public model

The conceptual public model is:

```rust
struct StructuredDocument {
    format: StructuredFormat,
    loaded_source: String,
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
```

`SourceSpan` is a zero-based, half-open UTF-8 byte range. Human-facing line
and column locations are one-based, with columns counted in Unicode scalar
values rather than bytes or display cells. Every span and source location
refers to `loaded_source`: the UTF-8 text supplied in `Chapter.content` after
mdBook's loading and BOM handling. Adapters normalize parser-native
coordinates at this boundary.

Sequence positions are represented by their ordered position, and mapping
entries retain insertion order. Decoded keys are unique strings. Numbers
retain their validated lexical form rather than being converted through
`f64`. Strings retain their decoded value exactly: the core performs no
trimming, case normalization, or simplification. The loaded source is retained
separately so comments, quoting, whitespace, line endings, and formatting
remain available to the [Original source view](html-presentation.md#original-source).

## Format adapters

Internally, format adapters satisfy a small parser seam equivalent to:

```rust
trait FormatAdapter {
    fn parse(
        &self,
        source: &str,
        source_name: &Path,
        limits: &Limits,
    ) -> Result<StructuredDocument, Diagnostic>;
}
```

The initial adapters use established Rust parser libraries that expose the
ordering and source provenance required by the public model. The JSON adapter
uses `json-syntax` 0.12.x. The YAML adapter delegates accepted syntax and
scalar interpretation to its selected parser library; v1 does not define a
separate YAML schema or an exhaustive YAML rejection matrix.

The selected YAML library must expose the typed scalar meaning and provenance
needed for a lossless projection. An event-only parser that leaves scalar
interpretation to its caller is insufficient by itself because it would force
the adapter to define scalar semantics instead of inheriting them.
Parser-library types remain private to their adapters.

After parsing succeeds, an adapter must project the parser result losslessly
into `StructuredDocument`. Duplicate decoded mapping keys are rejected so the
public mapping invariant remains unambiguous. Any other parsed construct that
the model cannot represent without coercion, omission, or simplification
causes an explicit projection diagnostic. The adapter must not silently
discard data or substitute a reduced representation merely to complete the
conversion.

## Limits and diagnostics

The parser and renderer enforce bounded input, node, and depth limits. The v1
defaults are:

```toml
max-input-bytes = 1048576
max-nodes = 10000
max-depth = 64
```

Limits cause a diagnostic before unbounded expansion or output generation. A
diagnostic identifies the source name, category, line and column when
available, and the structured key path when the parser can establish one.

The [verification strategy](verification.md#structured-core) treats this
public interface—not parser-specific representations—as the test surface.
