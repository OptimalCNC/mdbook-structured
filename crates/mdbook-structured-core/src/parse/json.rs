use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::path::Path;

use json_syntax::{CodeMap, Parse, Value};

use crate::model::ProvenanceError;
use crate::{
    Diagnostic, DiagnosticCategory, Limits, MappingEntry, Node, NodeValue, NumberLexeme,
    SourceLocation, SourceSpan, StructuredDocument, StructuredFormat, StructuredPath,
};

use super::FormatAdapter;
use super::coordinates::location_from_byte_offset;
use crate::limits::Budget;

pub(super) struct JsonAdapter;

impl FormatAdapter for JsonAdapter {
    fn parse(
        &self,
        loaded_source: &str,
        source_name: &Path,
        limits: Limits,
    ) -> Result<StructuredDocument, Diagnostic> {
        let (value, code_map) = match Value::parse_str(loaded_source) {
            Ok(parsed) => parsed,
            Err(error) => {
                let location = location_from_byte_offset(loaded_source, error.position())
                    .map_err(|provenance| invalid_provenance(source_name, None, provenance))?;
                return Err(Diagnostic::from_parts(
                    DiagnosticCategory::Parse,
                    source_name.to_owned(),
                    Some(location),
                    None,
                    error.to_string(),
                ));
            }
        };
        let mut budget = Budget::new(limits);
        let root = project_value(
            &value,
            &code_map,
            0,
            loaded_source,
            source_name,
            NonZeroUsize::MIN,
            &StructuredPath::root(),
            &mut budget,
        )?;

        Ok(StructuredDocument::from_parser_verified(
            StructuredFormat::Json,
            loaded_source.to_owned(),
            root,
            budget.finish(),
        ))
    }
}

#[allow(clippy::too_many_arguments)]
fn project_value(
    value: &Value,
    code_map: &CodeMap,
    fragment_index: usize,
    loaded_source: &str,
    source_name: &Path,
    depth: NonZeroUsize,
    path: &StructuredPath,
    budget: &mut Budget,
) -> Result<Node, Diagnostic> {
    let span = span_at(code_map, fragment_index, loaded_source, source_name, path)?;
    let location = budget
        .requires_diagnostic_location(depth)
        .then(|| location_from_byte_offset(loaded_source, span.start_byte()))
        .transpose()
        .map_err(|error| invalid_provenance(source_name, Some(path.clone()), error))?;
    budget.enter_node(depth, source_name, location, path)?;

    let value = match value {
        Value::Null => NodeValue::Null,
        Value::Boolean(value) => NodeValue::Boolean(*value),
        Value::Number(value) => NodeValue::Number(NumberLexeme::from_parser_verified(
            value.as_str().to_owned(),
        )),
        Value::String(value) => NodeValue::String(value.to_string()),
        Value::Array(values) => {
            let child_depth = next_depth(depth, source_name, path, location)?;
            let mut child_index = fragment_index + 1;
            let mut nodes = Vec::with_capacity(values.len());
            for (index, value) in values.iter().enumerate() {
                let child_path = path.clone().with_index(index);
                let child_fragment_count = fragment_count(value);
                nodes.push(project_value(
                    value,
                    code_map,
                    child_index,
                    loaded_source,
                    source_name,
                    child_depth,
                    &child_path,
                    budget,
                )?);
                child_index = child_index
                    .checked_add(child_fragment_count)
                    .ok_or_else(|| {
                        invalid_provenance(
                            source_name,
                            Some(child_path),
                            ProvenanceError::OutOfBounds,
                        )
                    })?;
            }
            NodeValue::Sequence(nodes)
        }
        Value::Object(object) => {
            let child_depth = next_depth(depth, source_name, path, location)?;
            let mut entry_index = fragment_index + 1;
            let mut decoded_keys = HashSet::with_capacity(object.len());
            let mut entries = Vec::with_capacity(object.len());
            for entry in object.entries() {
                let key_index = entry_index + 1;
                let value_index = entry_index + 2;
                let decoded_key = entry.key.to_string();
                let key_path = path.clone().with_key(decoded_key.clone());
                let entry_fragment_count = 2 + fragment_count(&entry.value);
                let key_span = span_at(code_map, key_index, loaded_source, source_name, &key_path)?;
                if !decoded_keys.insert(decoded_key.clone()) {
                    let key_location =
                        location_from_byte_offset(loaded_source, key_span.start_byte()).map_err(
                            |error| invalid_provenance(source_name, Some(key_path.clone()), error),
                        )?;
                    return Err(Diagnostic::from_parts(
                        DiagnosticCategory::DuplicateDecodedKey,
                        source_name.to_owned(),
                        Some(key_location),
                        Some(key_path),
                        format!("duplicate decoded mapping key {decoded_key:?}"),
                    ));
                }
                entries.push(MappingEntry::from_parser_verified(
                    decoded_key,
                    key_span,
                    project_value(
                        &entry.value,
                        code_map,
                        value_index,
                        loaded_source,
                        source_name,
                        child_depth,
                        &key_path,
                        budget,
                    )?,
                ));
                entry_index = entry_index
                    .checked_add(entry_fragment_count)
                    .ok_or_else(|| {
                        invalid_provenance(
                            source_name,
                            Some(key_path),
                            ProvenanceError::OutOfBounds,
                        )
                    })?;
            }
            NodeValue::Mapping(entries)
        }
    };

    Ok(Node::from_parser_verified(span, value))
}

fn span_at(
    code_map: &CodeMap,
    index: usize,
    loaded_source: &str,
    source_name: &Path,
    path: &StructuredPath,
) -> Result<SourceSpan, Diagnostic> {
    let entry = code_map.as_slice().get(index).ok_or_else(|| {
        invalid_provenance(
            source_name,
            Some(path.clone()),
            ProvenanceError::OutOfBounds,
        )
    })?;
    SourceSpan::try_from_parser(entry.span.start(), entry.span.end(), loaded_source)
        .map_err(|error| invalid_provenance(source_name, Some(path.clone()), error))
}

fn fragment_count(value: &Value) -> usize {
    match value {
        Value::Null | Value::Boolean(_) | Value::Number(_) | Value::String(_) => 1,
        Value::Array(values) => 1 + values.iter().map(fragment_count).sum::<usize>(),
        Value::Object(object) => {
            1 + object
                .entries()
                .iter()
                .map(|entry| 2 + fragment_count(&entry.value))
                .sum::<usize>()
        }
    }
}

fn next_depth(
    depth: NonZeroUsize,
    source_name: &Path,
    path: &StructuredPath,
    location: Option<SourceLocation>,
) -> Result<NonZeroUsize, Diagnostic> {
    depth
        .get()
        .checked_add(1)
        .and_then(NonZeroUsize::new)
        .ok_or_else(|| {
            Diagnostic::from_parts(
                DiagnosticCategory::DepthLimit,
                source_name.to_owned(),
                location,
                Some(path.clone()),
                "model depth exceeds the representation limit".to_owned(),
            )
        })
}

fn invalid_provenance(
    source_name: &Path,
    path: Option<StructuredPath>,
    error: ProvenanceError,
) -> Diagnostic {
    Diagnostic::from_parts(
        DiagnosticCategory::InvalidProvenance,
        source_name.to_owned(),
        None,
        path,
        format!("parser provenance is invalid: {error:?}"),
    )
}
