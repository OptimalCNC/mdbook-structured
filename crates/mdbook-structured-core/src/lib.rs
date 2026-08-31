//! Parser-neutral [`StructuredDocument`] values with JSON/YAML adapters retain
//! source provenance while providing typed diagnostics, resource limits, and
//! semantic HTML rendering.

mod diagnostic;
mod html;
mod limits;
mod model;
mod parse;

pub use diagnostic::{Diagnostic, DiagnosticCategory, PathSegment, SourceLocation, StructuredPath};
pub use html::{HtmlRenderOptions, RenderedHtml, render_structured_page};
pub use limits::{InvalidLimits, LimitName, Limits};
pub use model::{
    DocumentStats, MappingEntry, Node, NodeValue, NumberLexeme, SourceSpan, StructuredDocument,
    StructuredFormat,
};
pub use parse::parse_document;
