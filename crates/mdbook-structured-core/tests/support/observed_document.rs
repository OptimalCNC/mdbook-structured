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
    observe_raw_html(rendered.as_str())
}

pub(crate) fn observe_raw_html(rendered: &str) -> ObservedDocument {
    let parser = new_cmark_parser(rendered, &MarkdownOptions::default());
    let mut browser_html = String::new();
    pulldown_cmark::html::push_html(&mut browser_html, parser);
    let document = Html::parse_document(&browser_html);

    let roots = select_document(&document, ".structured-document");
    assert_eq!(roots.len(), 1, "expected exactly one structured root");
    let structured_root = roots[0];
    assert_eq!(
        structured_root.value().name(),
        "section",
        "structured root must be a section"
    );

    let semantic_children = direct_element_children(structured_root)
        .filter(|element| {
            element.value().name() == "h1"
                || element.value().attr("data-structured-action").is_some()
                || element.value().attr("data-structured-node").is_some()
                || element
                    .value()
                    .attr("data-structured-original-source")
                    .is_some()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        semantic_children.len(),
        5,
        "expected heading, two actions, model root, and original source as direct children"
    );
    let heading = semantic_children[0];
    assert_eq!(heading.value().name(), "h1", "heading must be first");

    let action_elements = &semantic_children[1..3];
    assert!(
        action_elements
            .iter()
            .all(|element| element.value().name() == "button"),
        "actions must be direct buttons after the heading"
    );
    assert_eq!(
        required_attribute(action_elements[0], "data-structured-action"),
        "expand-all",
        "expand-all action must precede collapse-all"
    );
    assert_eq!(
        required_attribute(action_elements[1], "data-structured-action"),
        "collapse-all",
        "collapse-all action must follow expand-all"
    );
    let actions = action_elements
        .iter()
        .copied()
        .map(
            |element| match required_attribute(element, "data-structured-action") {
                "expand-all" => "expand-all".to_owned(),
                "collapse-all" => "collapse-all".to_owned(),
                unknown => panic!("unknown structured action: {unknown}"),
            },
        )
        .collect::<Vec<_>>();

    let model_root = semantic_children[3];
    assert!(
        model_root.value().attr("data-structured-node").is_some(),
        "model root must directly follow the actions"
    );
    let original = semantic_children[4];
    assert!(
        original
            .value()
            .attr("data-structured-original-source")
            .is_some(),
        "original source must directly follow the model root"
    );
    assert_eq!(
        original.value().name(),
        "details",
        "original source must be a details element"
    );

    audit_model_node_ownership(structured_root, model_root);
    audit_empty_markers(structured_root);

    let original_children = direct_element_children(original).collect::<Vec<_>>();
    assert_eq!(
        original_children.len(),
        2,
        "original source must have only direct summary and pre children"
    );
    let summary = required_direct_summary(original, "original source");
    let pre = original_children[1];
    assert_eq!(
        pre.value().name(),
        "pre",
        "original source pre must directly follow its summary"
    );
    let code_children = direct_element_children(pre).collect::<Vec<_>>();
    assert_eq!(
        code_children.len(),
        1,
        "original source pre must have exactly one direct code child"
    );
    let code = code_children[0];
    assert_eq!(
        code.value().name(),
        "code",
        "original source pre child must be code"
    );
    required_attribute(code, "data-structured-format");

    ObservedDocument {
        heading: element_text(heading),
        actions,
        root: observe_root_node(model_root),
        original_open: original.value().attr("open").is_some(),
        original_summary: element_text(summary),
        original_format: required_attribute(code, "data-structured-format").to_owned(),
        original_source: element_text(code),
    }
}

fn observe_root_node(element: ElementRef<'_>) -> ObservedNode {
    match required_attribute(element, "data-structured-node") {
        "mapping" | "sequence" => {
            assert_eq!(
                element.value().name(),
                "div",
                "mapping and sequence roots must be non-disclosure div elements"
            );
            assert!(
                element.value().attr("data-structured-container").is_none(),
                "model root cannot carry the nested-container hook"
            );
            assert!(
                direct_element_children(element).all(|child| child.value().name() != "summary"),
                "model root cannot have a synthetic summary"
            );

            let children = observe_children(element);
            if required_attribute(element, "data-structured-node") == "mapping" {
                ObservedNode::Mapping {
                    label: None,
                    open: true,
                    children,
                }
            } else {
                ObservedNode::Sequence {
                    label: None,
                    open: true,
                    children,
                }
            }
        }
        "string" | "number" | "boolean" | "null" => observe_node(element),
        unknown => panic!("unknown structured root node: {unknown}"),
    }
}

fn observe_node(element: ElementRef<'_>) -> ObservedNode {
    match required_attribute(element, "data-structured-node") {
        "mapping" => {
            let summary = required_container_summary(element);
            ObservedNode::Mapping {
                label: observe_container_label(summary),
                open: element.value().attr("open").is_some(),
                children: observe_children(element),
            }
        }
        "sequence" => {
            let summary = required_container_summary(element);
            ObservedNode::Sequence {
                label: observe_container_label(summary),
                open: element.value().attr("open").is_some(),
                children: observe_children(element),
            }
        }
        scalar_kind @ ("string" | "number" | "boolean" | "null") => {
            assert!(
                element.value().attr("data-structured-container").is_none(),
                "scalar cannot carry the container hook"
            );
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
                label: observe_scalar_label(element),
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

fn required_container_summary(element: ElementRef<'_>) -> ElementRef<'_> {
    assert_eq!(
        element.value().name(),
        "details",
        "mapping and sequence nodes must be details elements"
    );
    assert!(
        element.value().attr("data-structured-container").is_some(),
        "mapping and sequence nodes must carry the container hook"
    );
    required_direct_summary(element, "model container")
}

fn required_direct_summary<'a>(element: ElementRef<'a>, owner: &str) -> ElementRef<'a> {
    let children = direct_element_children(element).collect::<Vec<_>>();
    let summaries = children
        .iter()
        .copied()
        .filter(|child| child.value().name() == "summary")
        .collect::<Vec<_>>();
    assert_eq!(
        summaries.len(),
        1,
        "{owner} must have exactly one direct summary"
    );
    assert_eq!(
        children.first().map(|child| child.value().name()),
        Some("summary"),
        "{owner} summary must be its first element child"
    );
    summaries[0]
}

fn observe_container_label(summary: ElementRef<'_>) -> Option<ObservedLabel> {
    let labels = direct_element_children(summary)
        .filter(|child| child.value().attr("data-structured-label").is_some())
        .collect::<Vec<_>>();
    assert!(labels.len() <= 1, "container can have at most one label");
    labels.first().copied().map(observe_label)
}

fn observe_scalar_label(element: ElementRef<'_>) -> Option<ObservedLabel> {
    let labels = direct_element_children(element)
        .filter(|child| child.value().attr("data-structured-label").is_some())
        .collect::<Vec<_>>();
    assert!(labels.len() <= 1, "scalar can have at most one label");
    labels.first().copied().map(observe_label)
}

fn observe_label(label: ElementRef<'_>) -> ObservedLabel {
    match required_attribute(label, "data-structured-label") {
        "key" => {
            let text = element_text(label);
            let empty_marker = match label.value().attr("data-structured-empty") {
                None => false,
                Some("key") if text.is_empty() => true,
                Some(unknown) => panic!("unknown or mismatched key empty marker: {unknown}"),
            };
            ObservedLabel::Key { text, empty_marker }
        }
        "index" => {
            assert!(
                label.value().attr("data-structured-empty").is_none(),
                "index labels cannot have empty markers"
            );
            ObservedLabel::Index(element_text(label).parse().expect("decimal index label"))
        }
        unknown => panic!("unknown structured label: {unknown}"),
    }
}

fn audit_model_node_ownership(structured_root: ElementRef<'_>, model_root: ElementRef<'_>) {
    for node in select_element(structured_root, "[data-structured-node]") {
        if node == model_root {
            continue;
        }
        let parent = parent_element(node).expect("non-root model node must have an element parent");
        assert!(
            matches!(
                parent.value().attr("data-structured-node"),
                Some("mapping" | "sequence")
            ),
            "non-root model node must be a direct child of a model container"
        );
        if parent == model_root {
            assert_eq!(
                parent.value().name(),
                "div",
                "model root must be a non-disclosure div"
            );
            assert!(
                parent.value().attr("data-structured-container").is_none(),
                "model root cannot carry the nested-container hook"
            );
        } else {
            assert_eq!(
                parent.value().name(),
                "details",
                "nested model-node parent must be a details container"
            );
            assert!(
                parent.value().attr("data-structured-container").is_some(),
                "nested model-node parent must carry the container hook"
            );
        }
    }
}

fn audit_empty_markers(structured_root: ElementRef<'_>) {
    if structured_root
        .value()
        .attr("data-structured-empty")
        .is_some()
    {
        audit_empty_marker(structured_root);
    }
    for marker in select_element(structured_root, "[data-structured-empty]") {
        audit_empty_marker(marker);
    }
}

fn audit_empty_marker(marker: ElementRef<'_>) {
    match required_attribute(marker, "data-structured-empty") {
        "key" => audit_empty_key_marker(marker),
        "string" => audit_empty_string_marker(marker),
        unknown => panic!("unknown structured empty marker: {unknown}"),
    }
}

fn audit_empty_key_marker(marker: ElementRef<'_>) {
    assert_eq!(
        marker.value().attr("data-structured-label"),
        Some("key"),
        "empty key marker must be owned by a key label"
    );
    assert!(element_text(marker).is_empty(), "marked key must be empty");

    let owner = label_owner_node(marker).expect("empty key label must belong to a model node");
    let mapping = parent_element(owner).expect("key-labelled node must have a mapping parent");
    assert_eq!(
        mapping.value().attr("data-structured-node"),
        Some("mapping"),
        "empty key marker must label a mapping child"
    );
}

fn audit_empty_string_marker(marker: ElementRef<'_>) {
    assert!(
        marker.value().attr("data-structured-value").is_some(),
        "empty string marker must be owned by a scalar value"
    );
    assert!(
        element_text(marker).is_empty(),
        "marked string value must be empty"
    );

    let owner = parent_element(marker).expect("empty string value must have a model-node parent");
    assert_eq!(
        owner.value().attr("data-structured-node"),
        Some("string"),
        "empty string marker must belong to a string node"
    );
}

fn label_owner_node(label: ElementRef<'_>) -> Option<ElementRef<'_>> {
    let parent = parent_element(label)?;
    if parent.value().attr("data-structured-node").is_some() {
        return Some(parent);
    }
    if parent.value().name() != "summary" {
        return None;
    }
    let owner = parent_element(parent)?;
    (owner.value().attr("data-structured-node").is_some()).then_some(owner)
}

fn parent_element(element: ElementRef<'_>) -> Option<ElementRef<'_>> {
    element.parent().and_then(ElementRef::wrap)
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
