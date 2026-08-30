use std::num::NonZeroUsize;
use std::path::Path;

use crate::{Diagnostic, DiagnosticCategory, DocumentStats, SourceLocation, StructuredPath};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    max_input_bytes: NonZeroUsize,
    max_nodes: NonZeroUsize,
    max_depth: NonZeroUsize,
}

impl Limits {
    pub const DEFAULT_MAX_INPUT_BYTES: usize = 1_048_576;
    pub const DEFAULT_MAX_NODES: usize = 10_000;
    pub const DEFAULT_MAX_DEPTH: usize = 64;

    pub fn try_new(
        max_input_bytes: usize,
        max_nodes: usize,
        max_depth: usize,
    ) -> Result<Self, InvalidLimits> {
        let max_input_bytes = NonZeroUsize::new(max_input_bytes).ok_or(InvalidLimits {
            zero_field: LimitName::MaxInputBytes,
        })?;
        let max_nodes = NonZeroUsize::new(max_nodes).ok_or(InvalidLimits {
            zero_field: LimitName::MaxNodes,
        })?;
        let max_depth = NonZeroUsize::new(max_depth).ok_or(InvalidLimits {
            zero_field: LimitName::MaxDepth,
        })?;

        Ok(Self {
            max_input_bytes,
            max_nodes,
            max_depth,
        })
    }

    pub fn max_input_bytes(&self) -> NonZeroUsize {
        self.max_input_bytes
    }

    pub fn max_nodes(&self) -> NonZeroUsize {
        self.max_nodes
    }

    pub fn max_depth(&self) -> NonZeroUsize {
        self.max_depth
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self::try_new(
            Self::DEFAULT_MAX_INPUT_BYTES,
            Self::DEFAULT_MAX_NODES,
            Self::DEFAULT_MAX_DEPTH,
        )
        .expect("default limits are positive")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LimitName {
    MaxInputBytes,
    MaxNodes,
    MaxDepth,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidLimits {
    zero_field: LimitName,
}

impl InvalidLimits {
    pub fn zero_field(&self) -> LimitName {
        self.zero_field
    }
}

pub(crate) struct Budget {
    limits: Limits,
    node_count: usize,
    max_depth_seen: usize,
}

impl Budget {
    pub(crate) fn new(limits: Limits) -> Self {
        Self {
            limits,
            node_count: 0,
            max_depth_seen: 0,
        }
    }

    pub(crate) fn enter_node(
        &mut self,
        depth: NonZeroUsize,
        source_name: &Path,
        location: Option<SourceLocation>,
        path: &StructuredPath,
    ) -> Result<(), Diagnostic> {
        let Some(next_node_count) = self.node_count.checked_add(1) else {
            return Err(Diagnostic::from_parts(
                DiagnosticCategory::NodeLimit,
                source_name.to_owned(),
                location,
                Some(path.clone()),
                "model node count exceeds the representation limit".to_owned(),
            ));
        };
        if next_node_count > self.limits.max_nodes.get() {
            return Err(Diagnostic::from_parts(
                DiagnosticCategory::NodeLimit,
                source_name.to_owned(),
                location,
                Some(path.clone()),
                format!(
                    "model node count {next_node_count} exceeds the configured limit {}",
                    self.limits.max_nodes
                ),
            ));
        }
        if depth > self.limits.max_depth {
            return Err(Diagnostic::from_parts(
                DiagnosticCategory::DepthLimit,
                source_name.to_owned(),
                location,
                Some(path.clone()),
                format!(
                    "model depth {depth} exceeds the configured limit {}",
                    self.limits.max_depth
                ),
            ));
        }

        self.node_count = next_node_count;
        self.max_depth_seen = self.max_depth_seen.max(depth.get());
        Ok(())
    }

    pub(crate) fn requires_diagnostic_location(&self, depth: NonZeroUsize) -> bool {
        self.node_count
            .checked_add(1)
            .is_none_or(|next_node_count| next_node_count > self.limits.max_nodes.get())
            || depth > self.limits.max_depth
    }

    pub(crate) fn finish(self) -> DocumentStats {
        DocumentStats::from_measured(
            NonZeroUsize::new(self.node_count).expect("a document has a root node"),
            NonZeroUsize::new(self.max_depth_seen).expect("a document has a root node"),
        )
    }
}
