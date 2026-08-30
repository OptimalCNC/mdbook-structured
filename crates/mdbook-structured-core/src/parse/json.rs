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
        if loaded_source.len() > limits.max_input_bytes().get() {
            return Err(Diagnostic::from_parts(
                DiagnosticCategory::InputByteLimit,
                source_name.to_owned(),
                None,
                Some(StructuredPath::root()),
                format!(
                    "input byte count {} exceeds the configured limit {}",
                    loaded_source.len(),
                    limits.max_input_bytes()
                ),
            ));
        }

        let (value, code_map) = Value::parse_str(loaded_source).map_err(|error| {
            Diagnostic::from_parts(
                DiagnosticCategory::Parse,
                source_name.to_owned(),
                None,
                None,
                error.to_string(),
            )
        })?;
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
    NonZeroUsize::new(depth.get().saturating_add(1)).ok_or_else(|| {
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{Limits, Node, NodeValue, NumberLexeme, PathSegment, StructuredDocument};
    use json_syntax::{Parse, Value};
    use proptest::prelude::*;
    use serde::Serialize;
    use serde::ser::{SerializeMap, Serializer};

    use super::super::coordinates::{
        location_from_byte_offset, reset_scalar_visits, scalar_visits,
    };
    use super::JsonAdapter;
    use crate::parse::FormatAdapter;

    #[test]
    fn positive_representative_preserves_shape_order_and_lexeme() {
        let source = include_str!("../../tests/fixtures/json/representative.json");
        let document = JsonAdapter
            .parse(source, Path::new("representative.json"), Limits::default())
            .unwrap();

        assert_eq!(document.loaded_source(), source);
        assert_eq!(document.stats().node_count().get(), 12);
        assert_eq!(document.stats().max_depth().get(), 4);
        assert_eq!(
            number_at(&document, &[key("amount")]).as_str(),
            "1.2300e+04"
        );
        assert_eq!(
            mapping_keys(document.root()),
            ["", "enabled", "text", "amount", "items"]
        );
    }

    #[test]
    fn positive_multibyte_spans_use_byte_offsets() {
        let source = include_str!("../../tests/fixtures/json/multibyte.json");
        let document = JsonAdapter
            .parse(source, Path::new("multibyte.json"), Limits::default())
            .unwrap();
        let after = number_at(&document, &[key("after")]);
        let (_, code_map) = Value::parse_str(source).unwrap();
        let after_offset = code_map.as_slice()[6].span.start();

        assert_eq!(
            node_at(&document, &[key("after")]).span().start_byte(),
            after_offset
        );
        assert_eq!(after.as_str(), "1");

        let location = location_from_byte_offset(source, after_offset).unwrap();
        assert_eq!((location.line().get(), location.column().get()), (1, 18));
    }

    #[test]
    fn positive_valid_projection_avoids_repeated_coordinate_prefix_scans() {
        let source = format!("{}[{}]", " ".repeat(4_096), vec!["0"; 64].join(","));
        reset_scalar_visits();

        let document = JsonAdapter
            .parse(
                &source,
                Path::new("leading-whitespace.json"),
                Limits::default(),
            )
            .unwrap();

        assert_eq!(document.stats().node_count().get(), 65);
        assert!(
            scalar_visits() <= source.chars().count(),
            "coordinate normalization visited {} scalars for a {}-scalar source",
            scalar_visits(),
            source.chars().count(),
        );
    }

    fn key(value: &str) -> PathSegment {
        PathSegment::Key(value.to_owned())
    }

    fn number_at<'a>(document: &'a StructuredDocument, path: &[PathSegment]) -> &'a NumberLexeme {
        let node = node_at(document, path);

        match node.value() {
            NodeValue::Number(value) => value,
            _ => panic!("path does not select a number"),
        }
    }

    fn node_at<'a>(document: &'a StructuredDocument, path: &[PathSegment]) -> &'a Node {
        let mut node = document.root();
        for segment in path {
            node = match (node.value(), segment) {
                (NodeValue::Mapping(entries), PathSegment::Key(key)) => entries
                    .iter()
                    .find(|entry| entry.decoded_key() == key)
                    .unwrap()
                    .value(),
                (NodeValue::Sequence(nodes), PathSegment::Index(index)) => &nodes[*index],
                _ => panic!("path does not select a node"),
            };
        }
        node
    }

    fn mapping_keys(node: &Node) -> Vec<&str> {
        match node.value() {
            NodeValue::Mapping(entries) => {
                entries.iter().map(|entry| entry.decoded_key()).collect()
            }
            _ => panic!("node is not a mapping"),
        }
    }

    proptest! {
        #[test]
        fn positive_generated_json_projects_losslessly(generated_tree in json_tree()) {
            let serialized = serde_json::to_string(&generated_tree).unwrap();
            let limits = Limits::try_new(
                serialized.len(),
                generated_tree.node_count(),
                generated_tree.max_depth(),
            ).unwrap();
            let document = JsonAdapter
                .parse(&serialized, Path::new("generated.json"), limits)
                .unwrap();

            prop_assert_eq!(observe_value(document.root()), generated_tree.clone());
            prop_assert_eq!(document.stats().node_count().get(), generated_tree.node_count());
            prop_assert_eq!(document.stats().max_depth().get(), generated_tree.max_depth());
            prop_assert_eq!(document.loaded_source(), serialized);
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum JsonTree {
        Mapping(Vec<(String, JsonTree)>),
        Sequence(Vec<JsonTree>),
        String(String),
        Number(i64),
        Boolean(bool),
        Null,
    }

    impl JsonTree {
        fn node_count(&self) -> usize {
            match self {
                Self::Mapping(entries) => {
                    1 + entries
                        .iter()
                        .map(|(_, value)| value.node_count())
                        .sum::<usize>()
                }
                Self::Sequence(values) => 1 + values.iter().map(Self::node_count).sum::<usize>(),
                Self::String(_) | Self::Number(_) | Self::Boolean(_) | Self::Null => 1,
            }
        }

        fn max_depth(&self) -> usize {
            match self {
                Self::Mapping(entries) if !entries.is_empty() => {
                    1 + entries
                        .iter()
                        .map(|(_, value)| value.max_depth())
                        .max()
                        .unwrap()
                }
                Self::Sequence(values) if !values.is_empty() => {
                    1 + values.iter().map(Self::max_depth).max().unwrap()
                }
                Self::Mapping(_)
                | Self::Sequence(_)
                | Self::String(_)
                | Self::Number(_)
                | Self::Boolean(_)
                | Self::Null => 1,
            }
        }
    }

    impl Serialize for JsonTree {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match self {
                Self::Mapping(entries) => {
                    let mut map = serializer.serialize_map(Some(entries.len()))?;
                    for (key, value) in entries {
                        map.serialize_entry(key, value)?;
                    }
                    map.end()
                }
                Self::Sequence(values) => values.serialize(serializer),
                Self::String(value) => serializer.serialize_str(value),
                Self::Number(value) => serializer.serialize_i64(*value),
                Self::Boolean(value) => serializer.serialize_bool(*value),
                Self::Null => serializer.serialize_unit(),
            }
        }
    }

    fn json_tree() -> impl Strategy<Value = JsonTree> {
        let leaf = prop_oneof![
            Just(JsonTree::Null),
            any::<bool>().prop_map(JsonTree::Boolean),
            any::<i16>().prop_map(|value| JsonTree::Number(value.into())),
            ascii_string().prop_map(JsonTree::String),
        ];
        leaf.prop_recursive(6, 256, 8, |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..=8).prop_map(JsonTree::Sequence),
                prop::collection::vec((ascii_string(), inner), 0..=8)
                    .prop_filter("mapping keys are unique", |entries| {
                        let mut keys = std::collections::HashSet::new();
                        entries.iter().all(|(key, _)| keys.insert(key))
                    })
                    .prop_map(JsonTree::Mapping),
            ]
        })
        .prop_filter("tree is within the node and depth bounds", |tree| {
            tree.node_count() <= 256 && tree.max_depth() <= 6
        })
    }

    fn ascii_string() -> impl Strategy<Value = String> {
        prop::collection::vec(32u8..=126, 0..=24)
            .prop_map(|bytes| String::from_utf8(bytes).expect("ASCII bytes are valid UTF-8"))
    }

    fn observe_value(node: &Node) -> JsonTree {
        match node.value() {
            NodeValue::Mapping(entries) => JsonTree::Mapping(
                entries
                    .iter()
                    .map(|entry| (entry.decoded_key().to_owned(), observe_value(entry.value())))
                    .collect(),
            ),
            NodeValue::Sequence(values) => {
                JsonTree::Sequence(values.iter().map(observe_value).collect())
            }
            NodeValue::String(value) => JsonTree::String(value.clone()),
            NodeValue::Number(value) => JsonTree::Number(value.as_str().parse().unwrap()),
            NodeValue::Boolean(value) => JsonTree::Boolean(*value),
            NodeValue::Null => JsonTree::Null,
        }
    }
}
