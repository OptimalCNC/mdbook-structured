use std::path::Path;

use mdbook_structured_core::{
    DiagnosticCategory, DocumentStats, Limits, Node, NodeValue, NumberLexeme, PathSegment,
    SourceLocation, StructuredDocument, StructuredFormat, StructuredPath, parse_document,
};

#[test]
fn limits_require_positive_values_and_keep_exact_ceilings() {
    assert!(Limits::try_new(0, 10, 10).is_err());
    let limits = Limits::try_new(1_048_576, 10_000, 64).unwrap();
    assert_eq!(limits.max_input_bytes().get(), 1_048_576);
    assert_eq!(limits.max_nodes().get(), 10_000);
    assert_eq!(limits.max_depth().get(), 64);
}

#[test]
fn locations_are_one_based_and_paths_are_typed() {
    let location = SourceLocation::try_new(3, 5).unwrap();
    assert_eq!((location.line().get(), location.column().get()), (3, 5));
    let path = StructuredPath::root()
        .with_key("services")
        .with_index(2)
        .with_key("port");
    assert_eq!(path.segments().len(), 3);
}

#[test]
fn structured_parse_representative_json_and_yaml_preserve_public_model() {
    let cases = [
        (
            StructuredFormat::Json,
            include_str!("fixtures/json/representative.json"),
            "representative.json",
        ),
        (
            StructuredFormat::Yaml,
            include_str!("fixtures/yaml/representative.yaml"),
            "representative.yaml",
        ),
    ];

    for (format, source, source_name) in cases {
        let document =
            parse_document(format, source, Path::new(source_name), Limits::default()).unwrap();

        assert_eq!(document.format(), format);
        assert_eq!(document.loaded_source(), source);
        assert_eq!(document.stats().node_count().get(), 12);
        assert_eq!(document.stats().max_depth().get(), 4);
        assert_eq!(
            mapping_keys(document.root()),
            ["", "enabled", "text", "amount", "items"]
        );
        assert_eq!(string_at(&document, &[key("")]), "");
        assert_eq!(boolean_at(&document, &[key("enabled")]), true);
        assert_eq!(string_at(&document, &[key("text")]), "true");
        assert_eq!(
            number_at(&document, &[key("amount")]).as_str(),
            "1.2300e+04"
        );
        assert!(matches!(
            node_at(&document, &[key("items"), PathSegment::Index(0)]).value(),
            NodeValue::Null
        ));
        assert_eq!(
            mapping_keys(node_at(&document, &[key("items"), PathSegment::Index(1)])),
            ["name", "flag", "empty", "count"]
        );
        assert_eq!(
            string_at(
                &document,
                &[key("items"), PathSegment::Index(1), key("name")]
            ),
            "x"
        );
        assert_eq!(
            boolean_at(
                &document,
                &[key("items"), PathSegment::Index(1), key("flag")]
            ),
            false
        );
        assert_eq!(
            string_at(
                &document,
                &[key("items"), PathSegment::Index(1), key("empty")]
            ),
            ""
        );
        assert_eq!(
            number_at(
                &document,
                &[key("items"), PathSegment::Index(1), key("count")]
            )
            .as_str(),
            "0"
        );
    }
}

#[test]
fn structured_parse_multibyte_spans_are_exact_utf8_byte_offsets() {
    let cases = [
        (
            StructuredFormat::Json,
            include_str!("fixtures/json/multibyte.json"),
            "multibyte.json",
            21,
            22,
        ),
        (
            StructuredFormat::Yaml,
            include_str!("fixtures/yaml/multibyte.yaml"),
            "multibyte.yaml",
            16,
            17,
        ),
    ];

    for (format, source, source_name, expected_start, expected_end) in cases {
        let document =
            parse_document(format, source, Path::new(source_name), Limits::default()).unwrap();
        let after = node_at(&document, &[key("after")]);

        assert_eq!(number_at(&document, &[key("after")]).as_str(), "1");
        assert_eq!(after.span().start_byte(), expected_start);
        assert_eq!(after.span().end_byte(), expected_end);
    }
}

#[test]
fn structured_parse_yaml_preserves_source_and_decodes_string_content() {
    let source = include_str!("fixtures/yaml/source-fidelity.yaml");
    let document = parse_document(
        StructuredFormat::Yaml,
        source,
        Path::new("source-fidelity.yaml"),
        Limits::default(),
    )
    .unwrap();

    assert_eq!(document.loaded_source(), source);
    assert_eq!(
        string_at(&document, &[key("quoted")]),
        "  keep both sides  "
    );
    assert_eq!(string_at(&document, &[key("block")]), "line one\nline two");
}

#[test]
fn structured_parse_yaml_preserves_inline_crlf_source() {
    let crlf_source = "first: 1\r\nsecond: 2\r\n";
    let document = parse_document(
        StructuredFormat::Yaml,
        crlf_source,
        Path::new("crlf.yaml"),
        Limits::default(),
    )
    .unwrap();

    assert_eq!(document.loaded_source(), crlf_source);
}

#[test]
fn failure_malformed_json_has_exact_parse_facts() {
    assert_failure(
        StructuredFormat::Json,
        r#"{"前":1,"broken": ]}"#,
        "malformed.json",
        Limits::default(),
        DiagnosticCategory::Parse,
        Some((1, 18)),
        None,
    );
}

#[test]
fn failure_malformed_yaml_has_exact_parse_facts() {
    assert_failure(
        StructuredFormat::Yaml,
        "outer:\n  broken: [1,\n",
        "malformed.yaml",
        Limits::default(),
        DiagnosticCategory::Parse,
        Some((3, 1)),
        None,
    );
}

#[test]
fn failure_duplicate_json_reports_second_decoded_key() {
    assert_failure(
        StructuredFormat::Json,
        "{\n  \"outer\": {\n    \"a\": 1,\n    \"\\u0061\": 2\n  }\n}\n",
        "duplicate.json",
        Limits::default(),
        DiagnosticCategory::DuplicateDecodedKey,
        Some((4, 5)),
        Some(vec![key("outer"), key("a")]),
    );
}

#[test]
fn failure_duplicate_yaml_reports_second_decoded_key() {
    assert_failure(
        StructuredFormat::Yaml,
        "outer:\n  a: 1\n  \"a\": 2\n",
        "duplicate.yaml",
        Limits::default(),
        DiagnosticCategory::DuplicateDecodedKey,
        Some((3, 3)),
        Some(vec![key("outer"), key("a")]),
    );
}

#[test]
fn failure_yaml_alias_reports_value_slot() {
    assert_failure(
        StructuredFormat::Yaml,
        "copy: *missing\n",
        "alias.yaml",
        Limits::default(),
        DiagnosticCategory::LosslessProjection,
        Some((1, 7)),
        Some(vec![key("copy")]),
    );
}

#[test]
fn failure_json_input_byte_limit_is_inclusive() {
    assert_pass_stats(
        StructuredFormat::Json,
        "0 ",
        "bytes.json",
        Limits::try_new(2, 16, 8).unwrap(),
        (1, 1),
    );
    assert_failure(
        StructuredFormat::Json,
        "0 ",
        "bytes.json",
        Limits::try_new(1, 16, 8).unwrap(),
        DiagnosticCategory::InputByteLimit,
        None,
        None,
    );
}

#[test]
fn failure_yaml_input_byte_limit_is_inclusive() {
    assert_pass_stats(
        StructuredFormat::Yaml,
        "0\n",
        "bytes.yaml",
        Limits::try_new(2, 16, 8).unwrap(),
        (1, 1),
    );
    assert_failure(
        StructuredFormat::Yaml,
        "0\n",
        "bytes.yaml",
        Limits::try_new(1, 16, 8).unwrap(),
        DiagnosticCategory::InputByteLimit,
        None,
        None,
    );
}

#[test]
fn failure_json_node_limit_is_inclusive() {
    assert_pass_stats(
        StructuredFormat::Json,
        r#"{"a":0,"b":1}"#,
        "nodes.json",
        Limits::try_new(64, 3, 8).unwrap(),
        (3, 2),
    );
    assert_failure(
        StructuredFormat::Json,
        r#"{"a":0,"b":1}"#,
        "nodes.json",
        Limits::try_new(64, 2, 8).unwrap(),
        DiagnosticCategory::NodeLimit,
        Some((1, 12)),
        Some(vec![key("b")]),
    );
}

#[test]
fn failure_yaml_node_limit_is_inclusive() {
    assert_pass_stats(
        StructuredFormat::Yaml,
        "a: 0\nb: 1\n",
        "nodes.yaml",
        Limits::try_new(64, 3, 8).unwrap(),
        (3, 2),
    );
    assert_failure(
        StructuredFormat::Yaml,
        "a: 0\nb: 1\n",
        "nodes.yaml",
        Limits::try_new(64, 2, 8).unwrap(),
        DiagnosticCategory::NodeLimit,
        Some((2, 4)),
        Some(vec![key("b")]),
    );
}

#[test]
fn failure_json_depth_limit_is_inclusive() {
    assert_pass_stats(
        StructuredFormat::Json,
        "[[0]]",
        "depth.json",
        Limits::try_new(64, 16, 3).unwrap(),
        (3, 3),
    );
    assert_failure(
        StructuredFormat::Json,
        "[[0]]",
        "depth.json",
        Limits::try_new(64, 16, 2).unwrap(),
        DiagnosticCategory::DepthLimit,
        Some((1, 3)),
        Some(vec![PathSegment::Index(0), PathSegment::Index(0)]),
    );
}

#[test]
fn failure_yaml_depth_limit_is_inclusive() {
    assert_pass_stats(
        StructuredFormat::Yaml,
        "- - 0\n",
        "depth.yaml",
        Limits::try_new(64, 16, 3).unwrap(),
        (3, 3),
    );
    assert_failure(
        StructuredFormat::Yaml,
        "- - 0\n",
        "depth.yaml",
        Limits::try_new(64, 16, 2).unwrap(),
        DiagnosticCategory::DepthLimit,
        Some((1, 5)),
        Some(vec![PathSegment::Index(0), PathSegment::Index(0)]),
    );
}

fn assert_pass_stats(
    format: StructuredFormat,
    source: &str,
    source_name: &str,
    limits: Limits,
    expected: (usize, usize),
) {
    let document = parse_document(format, source, Path::new(source_name), limits).unwrap();
    assert_stats(document.stats(), expected);
}

fn assert_stats(stats: DocumentStats, expected: (usize, usize)) {
    assert_eq!(stats.node_count().get(), expected.0);
    assert_eq!(stats.max_depth().get(), expected.1);
}

fn assert_failure(
    format: StructuredFormat,
    source: &str,
    source_name: &str,
    limits: Limits,
    expected_category: DiagnosticCategory,
    expected_location: Option<(usize, usize)>,
    expected_path: Option<Vec<PathSegment>>,
) {
    let diagnostic = parse_document(format, source, Path::new(source_name), limits).unwrap_err();

    assert_eq!(diagnostic.category(), expected_category);
    assert_eq!(diagnostic.source_name(), Path::new(source_name));
    assert_eq!(
        diagnostic
            .location()
            .map(|location| (location.line().get(), location.column().get())),
        expected_location
    );
    assert_eq!(
        diagnostic
            .structured_path()
            .map(|path| path.segments().to_vec()),
        expected_path
    );
}

fn key(value: &str) -> PathSegment {
    PathSegment::Key(value.to_owned())
}

fn number_at<'a>(document: &'a StructuredDocument, path: &[PathSegment]) -> &'a NumberLexeme {
    match node_at(document, path).value() {
        NodeValue::Number(value) => value,
        _ => panic!("path does not select a number"),
    }
}

fn string_at<'a>(document: &'a StructuredDocument, path: &[PathSegment]) -> &'a str {
    match node_at(document, path).value() {
        NodeValue::String(value) => value,
        _ => panic!("path does not select a string"),
    }
}

fn boolean_at(document: &StructuredDocument, path: &[PathSegment]) -> bool {
    match node_at(document, path).value() {
        NodeValue::Boolean(value) => *value,
        _ => panic!("path does not select a boolean"),
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
        NodeValue::Mapping(entries) => entries.iter().map(|entry| entry.decoded_key()).collect(),
        _ => panic!("node is not a mapping"),
    }
}
