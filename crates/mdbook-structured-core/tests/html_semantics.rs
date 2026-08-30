mod support;

use std::path::Path;

use mdbook_markdown::{MarkdownOptions, new_cmark_parser, pulldown_cmark};
use mdbook_structured_core::{
    HtmlRenderOptions, Limits, StructuredFormat, parse_document, render_structured_page,
};
use scraper::{Html, Selector};
use support::observed_document::{
    ObservedDocument, ObservedLabel, ObservedNode, ScalarKind, observe_raw_html,
    observe_rendered_html,
};

const REPRESENTATIVE_JSON: &str = r#"{
  "": "",
  "text": "  keep both sides and the complete long value\nline two  ",
  "amount": 1.2300e+04,
  "enabled": true,
  "nothing": null,
  "items": [
    {"nested": ["leaf"]},
    false
  ]
}"#;

const THRESHOLD_JSON: &str = r#"{"equal":[1,2],"over":[1,2,3]}"#;
const ZERO_THRESHOLD_YAML: &str = "empty: []\n";

const SAFETY_JSON: &str = r#"{
  "helper": "{{#include missing.md}}",
  "markup": "<img src=x onerror=alert(1)>",
  "multiline": "line one\nline two"
}"#;

const VALID_PROBE_HTML: &str = r#"<section class="structured-document"><h1>Probe</h1><button data-structured-action="expand-all">Expand all</button><button data-structured-action="collapse-all">Collapse all</button><details data-structured-container data-structured-node="mapping" open><summary>Mapping</summary><div data-structured-node="string"><span data-structured-label="key">name</span><span data-structured-value>value</span></div></details><details data-structured-original-source><summary>Original source (JSON)</summary><pre><code data-structured-format="json">&#123;"name":"value"&#125;</code></pre></details></section>"#;

#[test]
fn failure_probe_rejects_malformed_dom_structure_and_empty_hooks() {
    let cases = [
        (
            "nested heading",
            mutate_once(
                VALID_PROBE_HTML,
                "<h1>Probe</h1>",
                "<div><h1>Probe</h1></div>",
            ),
        ),
        (
            "nested actions",
            mutate_once(
                VALID_PROBE_HTML,
                "<button data-structured-action=\"expand-all\">Expand all</button><button data-structured-action=\"collapse-all\">Collapse all</button>",
                "<div><button data-structured-action=\"expand-all\">Expand all</button><button data-structured-action=\"collapse-all\">Collapse all</button></div>",
            ),
        ),
        (
            "non-details container",
            mutate_once(
                VALID_PROBE_HTML,
                "<details data-structured-container data-structured-node=\"mapping\" open><summary>Mapping</summary><div data-structured-node=\"string\">",
                "<div data-structured-container data-structured-node=\"mapping\" open><summary>Mapping</summary><div data-structured-node=\"string\">",
            )
            .replacen("</div></details><details data-structured-original-source>", "</div></div><details data-structured-original-source>", 1),
        ),
        (
            "missing container hook and direct summary",
            mutate_once(
                VALID_PROBE_HTML,
                "<details data-structured-container data-structured-node=\"mapping\" open><summary>Mapping</summary>",
                "<details data-structured-node=\"mapping\" open><div><summary>Mapping</summary></div>",
            ),
        ),
        (
            "nested original source",
            mutate_once(
                VALID_PROBE_HTML,
                "<details data-structured-original-source>",
                "<div><details data-structured-original-source>",
            )
            .replacen("</details></section>", "</details></div></section>", 1),
        ),
        (
            "original source before root",
            r#"<section class="structured-document"><h1>Probe</h1><button data-structured-action="expand-all">Expand all</button><button data-structured-action="collapse-all">Collapse all</button><details data-structured-original-source><summary>Original source (JSON)</summary><pre><code data-structured-format="json">&#123;"name":"value"&#125;</code></pre></details><details data-structured-container data-structured-node="mapping" open><summary>Mapping</summary><div data-structured-node="string"><span data-structured-label="key">name</span><span data-structured-value>value</span></div></details></section>"#.to_owned(),
        ),
        (
            "stray unknown empty hook",
            mutate_once(
                VALID_PROBE_HTML,
                "<span data-structured-value>value</span>",
                "<span data-structured-value>value</span><i data-structured-empty=\"unknown\"></i>",
            ),
        ),
        (
            "stray key marker on non-label",
            mutate_once(
                VALID_PROBE_HTML,
                "<span data-structured-value>value</span>",
                "<span data-structured-value>value</span><i data-structured-empty=\"key\"></i>",
            ),
        ),
        (
            "unknown empty hook on structured root",
            mutate_once(
                VALID_PROBE_HTML,
                "<section class=\"structured-document\">",
                "<section class=\"structured-document\" data-structured-empty=\"unknown\">",
            ),
        ),
    ];

    let accepted = cases
        .iter()
        .filter_map(|(name, html)| {
            std::panic::catch_unwind(|| observe_raw_html(html))
                .is_ok()
                .then_some(*name)
        })
        .collect::<Vec<_>>();

    assert!(
        accepted.is_empty(),
        "observer accepted malformed DOM cases: {accepted:?}"
    );
}

#[test]
fn positive_semantic_tree_preserves_types_order_disclosure_and_source() {
    let default_options = HtmlRenderOptions::default();
    assert_eq!(default_options.large_container_threshold(), 100);

    let document = parse_json(REPRESENTATIVE_JSON, "representative.json");
    let rendered = render_structured_page("Visible <chapter> & {name}", &document, default_options);

    assert_eq!(
        observe_rendered_html(&rendered),
        ObservedDocument {
            heading: "Visible <chapter> & {name}".to_owned(),
            actions: vec!["expand-all".to_owned(), "collapse-all".to_owned()],
            root: ObservedNode::Mapping {
                label: None,
                open: true,
                children: vec![
                    scalar_key("", true, ScalarKind::String, "", true),
                    scalar_key(
                        "text",
                        false,
                        ScalarKind::String,
                        "  keep both sides and the complete long value\nline two  ",
                        false,
                    ),
                    scalar_key("amount", false, ScalarKind::Number, "1.2300e+04", false),
                    scalar_key("enabled", false, ScalarKind::Boolean, "true", false),
                    scalar_key("nothing", false, ScalarKind::Null, "null", false),
                    ObservedNode::Sequence {
                        label: Some(key_label("items", false)),
                        open: true,
                        children: vec![
                            ObservedNode::Mapping {
                                label: Some(ObservedLabel::Index(0)),
                                open: false,
                                children: vec![ObservedNode::Sequence {
                                    label: Some(key_label("nested", false)),
                                    open: false,
                                    children: vec![scalar_index(
                                        0,
                                        ScalarKind::String,
                                        "leaf",
                                        false,
                                    )],
                                }],
                            },
                            scalar_index(1, ScalarKind::Boolean, "false", false),
                        ],
                    },
                ],
            },
            original_open: false,
            original_summary: "Original source (JSON)".to_owned(),
            original_format: "json".to_owned(),
            original_source: REPRESENTATIVE_JSON.to_owned(),
        }
    );

    let threshold_options = HtmlRenderOptions::new(2);
    assert_eq!(threshold_options.large_container_threshold(), 2);
    let threshold_document = parse_json(THRESHOLD_JSON, "threshold.json");
    let threshold_rendered =
        render_structured_page("Threshold", &threshold_document, threshold_options);
    assert_eq!(
        observe_rendered_html(&threshold_rendered).root,
        ObservedNode::Mapping {
            label: None,
            open: true,
            children: vec![
                ObservedNode::Sequence {
                    label: Some(key_label("equal", false)),
                    open: true,
                    children: vec![
                        scalar_index(0, ScalarKind::Number, "1", false),
                        scalar_index(1, ScalarKind::Number, "2", false),
                    ],
                },
                ObservedNode::Sequence {
                    label: Some(key_label("over", false)),
                    open: false,
                    children: vec![
                        scalar_index(0, ScalarKind::Number, "1", false),
                        scalar_index(1, ScalarKind::Number, "2", false),
                        scalar_index(2, ScalarKind::Number, "3", false),
                    ],
                },
            ],
        }
    );

    let zero_options = HtmlRenderOptions::new(0);
    assert_eq!(zero_options.large_container_threshold(), 0);
    let zero_document = parse_document(
        StructuredFormat::Yaml,
        ZERO_THRESHOLD_YAML,
        Path::new("zero.yaml"),
        Limits::default(),
    )
    .unwrap();
    let zero_rendered = render_structured_page("Zero", &zero_document, zero_options);
    let zero_observed = observe_rendered_html(&zero_rendered);
    assert_eq!(
        zero_observed.root,
        ObservedNode::Mapping {
            label: None,
            open: false,
            children: vec![ObservedNode::Sequence {
                label: Some(key_label("empty", false)),
                open: true,
                children: vec![],
            }],
        }
    );
    assert_eq!(zero_observed.original_summary, "Original source (YAML)");
    assert_eq!(zero_observed.original_format, "yaml");
    assert_eq!(zero_observed.original_source, ZERO_THRESHOLD_YAML);
}

#[test]
fn positive_raw_block_encodes_helpers_markup_and_browser_text_safely() {
    let document = parse_json(SAFETY_JSON, "safety.json");
    let rendered = render_structured_page("Safety", &document, HtmlRenderOptions::default());

    let events =
        new_cmark_parser(rendered.as_str(), &MarkdownOptions::default()).collect::<Vec<_>>();
    let raw_html = match events.as_slice() {
        [
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::HtmlBlock),
            pulldown_cmark::Event::Html(html),
            pulldown_cmark::Event::End(pulldown_cmark::TagEnd::HtmlBlock),
        ] => html.as_ref(),
        events => panic!("expected one uninterrupted raw HTML block, got {events:#?}"),
    };
    assert!(
        !raw_html.contains("{{#include missing.md}}"),
        "literal helper syntax must not reach the raw block"
    );

    let observed = observe_rendered_html(&rendered);
    assert_eq!(
        observed.root,
        ObservedNode::Mapping {
            label: None,
            open: true,
            children: vec![
                scalar_key(
                    "helper",
                    false,
                    ScalarKind::String,
                    "{{#include missing.md}}",
                    false,
                ),
                scalar_key(
                    "markup",
                    false,
                    ScalarKind::String,
                    "<img src=x onerror=alert(1)>",
                    false,
                ),
                scalar_key(
                    "multiline",
                    false,
                    ScalarKind::String,
                    "line one\nline two",
                    false,
                ),
            ],
        }
    );
    assert_eq!(observed.original_source, SAFETY_JSON);

    let mut browser_html = String::new();
    pulldown_cmark::html::push_html(
        &mut browser_html,
        new_cmark_parser(rendered.as_str(), &MarkdownOptions::default()),
    );
    let html = Html::parse_document(&browser_html);
    assert_eq!(html.select(&Selector::parse("img").unwrap()).count(), 0);
    let source_code = html
        .select(&Selector::parse("pre > code[data-structured-format]").unwrap())
        .collect::<Vec<_>>();
    assert_eq!(source_code.len(), 1);
}

fn parse_json(source: &str, source_name: &str) -> mdbook_structured_core::StructuredDocument {
    parse_document(
        StructuredFormat::Json,
        source,
        Path::new(source_name),
        Limits::default(),
    )
    .unwrap()
}

fn key_label(text: &str, empty_marker: bool) -> ObservedLabel {
    ObservedLabel::Key {
        text: text.to_owned(),
        empty_marker,
    }
}

fn scalar_key(
    key: &str,
    empty_key_marker: bool,
    kind: ScalarKind,
    text: &str,
    empty_string_marker: bool,
) -> ObservedNode {
    ObservedNode::Scalar {
        label: Some(key_label(key, empty_key_marker)),
        kind,
        text: text.to_owned(),
        empty_string_marker,
    }
}

fn scalar_index(
    index: usize,
    kind: ScalarKind,
    text: &str,
    empty_string_marker: bool,
) -> ObservedNode {
    ObservedNode::Scalar {
        label: Some(ObservedLabel::Index(index)),
        kind,
        text: text.to_owned(),
        empty_string_marker,
    }
}

fn mutate_once(source: &str, from: &str, to: &str) -> String {
    assert_eq!(
        source.matches(from).count(),
        1,
        "mutation target must occur exactly once"
    );
    source.replacen(from, to, 1)
}
