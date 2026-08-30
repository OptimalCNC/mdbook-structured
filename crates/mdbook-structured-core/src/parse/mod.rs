mod coordinates;
mod json;

use std::path::Path;

use crate::{Diagnostic, Limits, StructuredDocument};

pub(crate) trait FormatAdapter {
    fn parse(
        &self,
        loaded_source: &str,
        source_name: &Path,
        limits: Limits,
    ) -> Result<StructuredDocument, Diagnostic>;
}
