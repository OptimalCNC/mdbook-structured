use mdbook_markdown::{MarkdownOptions, new_cmark_parser, pulldown_cmark};
use mdbook_structured_core::RenderedHtml;
use scraper::{ElementRef, Html, Selector};

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ObservedDocument {
    pub(crate) heading: String,
    pub(crate) actions: Vec<String>,
    pub(crate) root: ObservedNode,
    pub(crate) original_open: bool,
    pub(crate) original_summary: String,
    pub(crate) original_format: String,
    pub(crate) original_source: String,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ObservedNode {
    Mapping {
        label: Option<ObservedLabel>,
        open: bool,
        children: Vec<ObservedNode>,
    },
    Sequence {
        label: Option<ObservedLabel>,
        open: bool,
        children: Vec<ObservedNode>,
    },
    Scalar {
        label: Option<ObservedLabel>,
        kind: ScalarKind,
        text: String,
        empty_string_marker: bool,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ObservedLabel {
    Key { text: String, empty_marker: bool },
    Index(usize),
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ScalarKind {
    String,
    Number,
    Boolean,
    Null,
}

pub(crate) fn observe_rendered_html(rendered: &RenderedHtml) -> ObservedDocument {
    let parser = new_cmark_parser(rendered.as_str(), &MarkdownOptions::default());
    let mut browser_html = String::new();
    pulldown_cmark::html::push_html(&mut browser_html, parser);
    let document = Html::parse_document(&browser_html);

    let roots = select_document(&document, ".structured-document");
    assert_eq!(roots.len(), 1, "expected exactly one structured root");
    let structured_root = roots[0];

    let headings = select_element(structured_root, "h1");
    assert_eq!(headings.len(), 1, "expected exactly one visible heading");

    let actions = select_element(structured_root, "[data-structured-action]")
        .into_iter()
        .map(
            |element| match required_attribute(element, "data-structured-action") {
                "expand-all" => "expand-all".to_owned(),
                "collapse-all" => "collapse-all".to_owned(),
                unknown => panic!("unknown structured action: {unknown}"),
            },
        )
        .collect::<Vec<_>>();

    let model_roots = direct_element_children(structured_root)
        .filter(|element| element.value().attr("data-structured-node").is_some())
        .collect::<Vec<_>>();
    assert_eq!(model_roots.len(), 1, "expected a direct model root");

    let originals = select_element(structured_root, "details[data-structured-original-source]");
    assert_eq!(originals.len(), 1, "expected exactly one original source");
    let original = originals[0];
    let summary = direct_element_children(original)
        .find(|element| element.value().name() == "summary")
        .expect("original source must have a summary");
    let codes = select_element(original, "code[data-structured-format]");
    assert_eq!(
        codes.len(),
        1,
        "expected exactly one format-labelled source"
    );
    let code = codes[0];
    let parent = code
        .parent()
        .and_then(ElementRef::wrap)
        .expect("source code must have an element parent");
    assert_eq!(
        parent.value().name(),
        "pre",
        "source code must be a child of pre"
    );

    ObservedDocument {
        heading: element_text(headings[0]),
        actions,
        root: observe_node(model_roots[0]),
        original_open: original.value().attr("open").is_some(),
        original_summary: element_text(summary),
        original_format: required_attribute(code, "data-structured-format").to_owned(),
        original_source: element_text(code),
    }
}

fn observe_node(element: ElementRef<'_>) -> ObservedNode {
    let label = observe_label(element);
    match required_attribute(element, "data-structured-node") {
        "mapping" => ObservedNode::Mapping {
            label,
            open: element.value().attr("open").is_some(),
            children: observe_children(element),
        },
        "sequence" => ObservedNode::Sequence {
            label,
            open: element.value().attr("open").is_some(),
            children: observe_children(element),
        },
        scalar_kind @ ("string" | "number" | "boolean" | "null") => {
            let values = direct_element_children(element)
                .filter(|child| child.value().attr("data-structured-value").is_some())
                .collect::<Vec<_>>();
            assert_eq!(values.len(), 1, "scalar must have exactly one value");
            let value = values[0];
            let text = element_text(value);
            let empty_string_marker = match value.value().attr("data-structured-empty") {
                None => false,
                Some("string") if scalar_kind == "string" && text.is_empty() => true,
                Some(unknown) => panic!("unknown or mismatched scalar empty marker: {unknown}"),
            };

            ObservedNode::Scalar {
                label,
                kind: match scalar_kind {
                    "string" => ScalarKind::String,
                    "number" => ScalarKind::Number,
                    "boolean" => ScalarKind::Boolean,
                    "null" => ScalarKind::Null,
                    _ => unreachable!(),
                },
                text,
                empty_string_marker,
            }
        }
        unknown => panic!("unknown structured node: {unknown}"),
    }
}

fn observe_children(element: ElementRef<'_>) -> Vec<ObservedNode> {
    direct_element_children(element)
        .filter(|child| child.value().attr("data-structured-node").is_some())
        .map(observe_node)
        .collect()
}

fn observe_label(element: ElementRef<'_>) -> Option<ObservedLabel> {
    let label = if element.value().name() == "details" {
        direct_element_children(element)
            .find(|child| child.value().name() == "summary")
            .and_then(|summary| {
                direct_element_children(summary)
                    .find(|child| child.value().attr("data-structured-label").is_some())
            })
    } else {
        direct_element_children(element)
            .find(|child| child.value().attr("data-structured-label").is_some())
    }?;

    match required_attribute(label, "data-structured-label") {
        "key" => {
            let text = element_text(label);
            let empty_marker = match label.value().attr("data-structured-empty") {
                None => false,
                Some("key") if text.is_empty() => true,
                Some(unknown) => panic!("unknown or mismatched key empty marker: {unknown}"),
            };
            Some(ObservedLabel::Key { text, empty_marker })
        }
        "index" => {
            assert!(
                label.value().attr("data-structured-empty").is_none(),
                "index labels cannot have empty markers"
            );
            Some(ObservedLabel::Index(
                element_text(label).parse().expect("decimal index label"),
            ))
        }
        unknown => panic!("unknown structured label: {unknown}"),
    }
}

fn direct_element_children(element: ElementRef<'_>) -> impl Iterator<Item = ElementRef<'_>> {
    element.children().filter_map(ElementRef::wrap)
}

fn element_text(element: ElementRef<'_>) -> String {
    element.text().collect()
}

fn required_attribute<'a>(element: ElementRef<'a>, name: &str) -> &'a str {
    element
        .value()
        .attr(name)
        .unwrap_or_else(|| panic!("missing required attribute: {name}"))
}

fn select_document<'a>(scope: &'a Html, selector: &str) -> Vec<ElementRef<'a>> {
    let selector = Selector::parse(selector).expect("test-owned stable selector");
    scope.select(&selector).collect()
}

fn select_element<'a>(scope: ElementRef<'a>, selector: &str) -> Vec<ElementRef<'a>> {
    let selector = Selector::parse(selector).expect("test-owned stable selector");
    scope.select(&selector).collect()
}
