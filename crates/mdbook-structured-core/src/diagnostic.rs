#![allow(dead_code)]

use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticCategory {
    InputByteLimit,
    NodeLimit,
    DepthLimit,
    Parse,
    DuplicateDecodedKey,
    LosslessProjection,
    InvalidProvenance,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceLocation {
    line: NonZeroUsize,
    column: NonZeroUsize,
}

impl SourceLocation {
    pub fn try_new(line: usize, column: usize) -> Option<Self> {
        Some(Self {
            line: NonZeroUsize::new(line)?,
            column: NonZeroUsize::new(column)?,
        })
    }

    pub fn line(&self) -> NonZeroUsize {
        self.line
    }

    pub fn column(&self) -> NonZeroUsize {
        self.column
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuredPath(Vec<PathSegment>);

impl StructuredPath {
    pub fn root() -> Self {
        Self(Vec::new())
    }

    pub fn with_key(self, key: impl Into<String>) -> Self {
        let mut segments = self.0;
        segments.push(PathSegment::Key(key.into()));
        Self(segments)
    }

    pub fn with_index(self, index: usize) -> Self {
        let mut segments = self.0;
        segments.push(PathSegment::Index(index));
        Self(segments)
    }

    pub fn segments(&self) -> &[PathSegment] {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PathSegment {
    Key(String),
    Index(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    category: DiagnosticCategory,
    source_name: PathBuf,
    location: Option<SourceLocation>,
    structured_path: Option<StructuredPath>,
    detail: String,
}

impl Diagnostic {
    pub(crate) fn from_parts(
        category: DiagnosticCategory,
        source_name: PathBuf,
        location: Option<SourceLocation>,
        structured_path: Option<StructuredPath>,
        detail: String,
    ) -> Self {
        Self {
            category,
            source_name,
            location,
            structured_path,
            detail,
        }
    }

    pub fn category(&self) -> DiagnosticCategory {
        self.category
    }

    pub fn source_name(&self) -> &Path {
        &self.source_name
    }

    pub fn location(&self) -> Option<SourceLocation> {
        self.location
    }

    pub fn structured_path(&self) -> Option<&StructuredPath> {
        self.structured_path.as_ref()
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}
