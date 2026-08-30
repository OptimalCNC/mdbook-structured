#![allow(dead_code)]

use std::num::NonZeroUsize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuredFormat {
    Json,
    Yaml,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumberLexeme(Box<str>);

impl NumberLexeme {
    pub(crate) fn from_parser_verified(value: impl Into<Box<str>>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceSpan {
    start_byte: usize,
    end_byte: usize,
}

impl SourceSpan {
    pub(crate) fn try_from_parser(
        start_byte: usize,
        end_byte: usize,
        loaded_source: &str,
    ) -> Result<Self, ProvenanceError> {
        if start_byte > end_byte {
            return Err(ProvenanceError::Reversed);
        }
        if end_byte > loaded_source.len() {
            return Err(ProvenanceError::OutOfBounds);
        }
        if !loaded_source.is_char_boundary(start_byte) || !loaded_source.is_char_boundary(end_byte)
        {
            return Err(ProvenanceError::NotCharBoundary);
        }

        Ok(Self {
            start_byte,
            end_byte,
        })
    }

    pub fn start_byte(&self) -> usize {
        self.start_byte
    }

    pub fn end_byte(&self) -> usize {
        self.end_byte
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProvenanceError {
    Reversed,
    OutOfBounds,
    NotCharBoundary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuredDocument {
    format: StructuredFormat,
    loaded_source: String,
    root: Node,
    stats: DocumentStats,
}

impl StructuredDocument {
    pub(crate) fn from_parser_verified(
        format: StructuredFormat,
        loaded_source: String,
        root: Node,
        stats: DocumentStats,
    ) -> Self {
        Self {
            format,
            loaded_source,
            root,
            stats,
        }
    }

    pub fn format(&self) -> StructuredFormat {
        self.format
    }

    pub fn loaded_source(&self) -> &str {
        &self.loaded_source
    }

    pub fn root(&self) -> &Node {
        &self.root
    }

    pub fn stats(&self) -> DocumentStats {
        self.stats
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Node {
    span: SourceSpan,
    value: NodeValue,
}

impl Node {
    pub(crate) fn from_parser_verified(span: SourceSpan, value: NodeValue) -> Self {
        Self { span, value }
    }

    pub fn span(&self) -> SourceSpan {
        self.span
    }

    pub fn value(&self) -> &NodeValue {
        &self.value
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeValue {
    Mapping(Vec<MappingEntry>),
    Sequence(Vec<Node>),
    String(String),
    Number(NumberLexeme),
    Boolean(bool),
    Null,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MappingEntry {
    decoded_key: String,
    key_span: SourceSpan,
    value: Node,
}

impl MappingEntry {
    pub(crate) fn from_parser_verified(
        decoded_key: String,
        key_span: SourceSpan,
        value: Node,
    ) -> Self {
        Self {
            decoded_key,
            key_span,
            value,
        }
    }

    pub fn decoded_key(&self) -> &str {
        &self.decoded_key
    }

    pub fn key_span(&self) -> SourceSpan {
        self.key_span
    }

    pub fn value(&self) -> &Node {
        &self.value
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentStats {
    node_count: NonZeroUsize,
    max_depth: NonZeroUsize,
}

impl DocumentStats {
    pub(crate) fn from_measured(node_count: NonZeroUsize, max_depth: NonZeroUsize) -> Self {
        Self {
            node_count,
            max_depth,
        }
    }

    pub fn node_count(&self) -> NonZeroUsize {
        self.node_count
    }

    pub fn max_depth(&self) -> NonZeroUsize {
        self.max_depth
    }
}
