use std::path::Path;

use mdbook_structured_core::{Limits, Node, NodeValue, StructuredFormat, parse_document};
use proptest::prelude::*;
use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};

proptest! {
    #[test]
    fn positive_generated_json_projects_losslessly(generated_tree in json_tree()) {
        let serialized = serde_json::to_string(&generated_tree).unwrap();
        let limits = Limits::try_new(
            serialized.len(),
            generated_tree.node_count(),
            generated_tree.max_depth(),
        ).unwrap();
        let document = parse_document(
            StructuredFormat::Json,
            &serialized,
            Path::new("generated.json"),
            limits,
        ).unwrap();

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
