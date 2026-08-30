#[cfg_attr(test, allow(dead_code))]
mod coordinates;
mod json;
mod yaml;

use std::path::Path;

use crate::{Diagnostic, DiagnosticCategory, Limits, StructuredDocument, StructuredFormat};

use self::json::JsonAdapter;
use self::yaml::YamlAdapter;

pub(crate) trait FormatAdapter {
    fn parse(
        &self,
        loaded_source: &str,
        source_name: &Path,
        limits: Limits,
    ) -> Result<StructuredDocument, Diagnostic>;
}

pub fn parse_document(
    format: StructuredFormat,
    loaded_source: &str,
    source_name: &Path,
    limits: Limits,
) -> Result<StructuredDocument, Diagnostic> {
    if loaded_source.len() > limits.max_input_bytes().get() {
        return Err(Diagnostic::from_parts(
            DiagnosticCategory::InputByteLimit,
            source_name.to_owned(),
            None,
            None,
            format!(
                "input byte count {} exceeds the configured limit {}",
                loaded_source.len(),
                limits.max_input_bytes()
            ),
        ));
    }

    match format {
        StructuredFormat::Json => JsonAdapter.parse(loaded_source, source_name, limits),
        StructuredFormat::Yaml => YamlAdapter.parse(loaded_source, source_name, limits),
    }
}
