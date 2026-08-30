use std::num::NonZeroUsize;

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
