mod encode;

use crate::{Node, NodeValue, StructuredDocument, StructuredFormat};

use self::encode::{push_encoded_attribute, push_encoded_text};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HtmlRenderOptions {
    large_container_threshold: usize,
}

impl HtmlRenderOptions {
    pub fn new(large_container_threshold: usize) -> Self {
        Self {
            large_container_threshold,
        }
    }

    pub fn large_container_threshold(&self) -> usize {
        self.large_container_threshold
    }
}

impl Default for HtmlRenderOptions {
    fn default() -> Self {
        Self::new(100)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedHtml(String);

impl RenderedHtml {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn render_structured_page(
    chapter_name: &str,
    document: &StructuredDocument,
    options: HtmlRenderOptions,
) -> RenderedHtml {
    let mut output = String::new();
    output.push_str("<section class=\"structured-document\"><h1>");
    push_encoded_text(&mut output, chapter_name);
    output.push_str("</h1>");
    push_action(&mut output, "expand-all", "Expand all");
    push_action(&mut output, "collapse-all", "Collapse all");
    push_node(&mut output, document.root(), None, 1, options);
    push_original_source(&mut output, document);
    output.push_str("</section>");
    RenderedHtml(output)
}

#[derive(Clone, Copy)]
enum NodeLabel<'a> {
    Key(&'a str),
    Index(usize),
}

fn push_action(output: &mut String, action: &str, visible_text: &str) {
    output.push_str("<button type=\"button\" data-structured-action=\"");
    push_encoded_attribute(output, action);
    output.push_str("\">");
    push_encoded_text(output, visible_text);
    output.push_str("</button>");
}

fn push_node(
    output: &mut String,
    node: &Node,
    label: Option<NodeLabel<'_>>,
    depth: usize,
    options: HtmlRenderOptions,
) {
    match node.value() {
        NodeValue::Mapping(entries) => {
            push_container_start(output, "mapping", label, depth, entries.len(), options);
            for entry in entries {
                push_node(
                    output,
                    entry.value(),
                    Some(NodeLabel::Key(entry.decoded_key())),
                    depth + 1,
                    options,
                );
            }
            output.push_str("</details>");
        }
        NodeValue::Sequence(items) => {
            push_container_start(output, "sequence", label, depth, items.len(), options);
            for (index, item) in items.iter().enumerate() {
                push_node(
                    output,
                    item,
                    Some(NodeLabel::Index(index)),
                    depth + 1,
                    options,
                );
            }
            output.push_str("</details>");
        }
        NodeValue::String(value) => {
            push_scalar(output, "string", label, value, value.is_empty());
        }
        NodeValue::Number(value) => {
            push_scalar(output, "number", label, value.as_str(), false);
        }
        NodeValue::Boolean(value) => {
            push_scalar(
                output,
                "boolean",
                label,
                if *value { "true" } else { "false" },
                false,
            );
        }
        NodeValue::Null => push_scalar(output, "null", label, "null", false),
    }
}

fn push_container_start(
    output: &mut String,
    kind: &str,
    label: Option<NodeLabel<'_>>,
    depth: usize,
    immediate_child_count: usize,
    options: HtmlRenderOptions,
) {
    output.push_str("<details data-structured-container data-structured-node=\"");
    push_encoded_attribute(output, kind);
    output.push('"');
    if depth <= 2 && immediate_child_count <= options.large_container_threshold() {
        output.push_str(" open");
    }
    output.push_str("><summary>");
    match label {
        Some(label) => push_label(output, label),
        None => push_encoded_text(
            output,
            if kind == "mapping" {
                "Mapping"
            } else {
                "Sequence"
            },
        ),
    }
    output.push_str("</summary>");
}

fn push_scalar(
    output: &mut String,
    kind: &str,
    label: Option<NodeLabel<'_>>,
    value: &str,
    empty_string: bool,
) {
    output.push_str("<div data-structured-node=\"");
    push_encoded_attribute(output, kind);
    output.push_str("\">");
    if let Some(label) = label {
        push_label(output, label);
    }
    output.push_str("<span data-structured-value");
    if empty_string {
        output.push_str(" data-structured-empty=\"string\"");
    }
    output.push('>');
    push_encoded_text(output, value);
    output.push_str("</span></div>");
}

fn push_label(output: &mut String, label: NodeLabel<'_>) {
    match label {
        NodeLabel::Key(key) => {
            output.push_str("<span data-structured-label=\"key\"");
            if key.is_empty() {
                output.push_str(" data-structured-empty=\"key\"");
            }
            output.push('>');
            push_encoded_text(output, key);
        }
        NodeLabel::Index(index) => {
            output.push_str("<span data-structured-label=\"index\">");
            push_encoded_text(output, &index.to_string());
        }
    }
    output.push_str("</span>");
}

fn push_original_source(output: &mut String, document: &StructuredDocument) {
    let (format_attribute, format_label) = match document.format() {
        StructuredFormat::Json => ("json", "JSON"),
        StructuredFormat::Yaml => ("yaml", "YAML"),
    };
    output.push_str("<details data-structured-original-source><summary>Original source (");
    push_encoded_text(output, format_label);
    output.push_str(")</summary><pre><code data-structured-format=\"");
    push_encoded_attribute(output, format_attribute);
    output.push_str("\">");
    push_encoded_text(output, document.loaded_source());
    output.push_str("</code></pre></details>");
}
