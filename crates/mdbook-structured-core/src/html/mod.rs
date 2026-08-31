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
    push_node(&mut output, document.root(), NodePosition::Root, 1, options);
    push_original_source(&mut output, document);
    output.push_str("</section>");
    RenderedHtml(output)
}

#[derive(Clone, Copy)]
enum NodeLabel<'a> {
    Key(&'a str),
    Index(usize),
}

#[derive(Clone, Copy)]
enum NodePosition<'a> {
    Root,
    Child(NodeLabel<'a>),
}

#[derive(Clone, Copy)]
enum ScalarKind {
    String,
    Number,
    Boolean,
    Null,
}

impl ScalarKind {
    fn attribute_value(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Null => "null",
        }
    }
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
    position: NodePosition<'_>,
    depth: usize,
    options: HtmlRenderOptions,
) {
    match node.value() {
        NodeValue::Mapping(entries) => {
            push_container_start(
                output,
                "mapping",
                position,
                depth,
                entries.len(),
                entries.iter().all(|entry| is_scalar(entry.value().value())),
                options,
            );
            for entry in entries {
                push_node(
                    output,
                    entry.value(),
                    NodePosition::Child(NodeLabel::Key(entry.decoded_key())),
                    depth + 1,
                    options,
                );
            }
            output.push_str(match position {
                NodePosition::Root => "</div>",
                NodePosition::Child(_) => "</details>",
            });
        }
        NodeValue::Sequence(items) => {
            push_container_start(
                output,
                "sequence",
                position,
                depth,
                items.len(),
                items.iter().all(|item| is_scalar(item.value())),
                options,
            );
            for (index, item) in items.iter().enumerate() {
                push_node(
                    output,
                    item,
                    NodePosition::Child(NodeLabel::Index(index)),
                    depth + 1,
                    options,
                );
            }
            output.push_str(match position {
                NodePosition::Root => "</div>",
                NodePosition::Child(_) => "</details>",
            });
        }
        NodeValue::String(value) => {
            push_scalar(
                output,
                ScalarKind::String,
                position,
                value,
                value.is_empty(),
            );
        }
        NodeValue::Number(value) => {
            push_scalar(output, ScalarKind::Number, position, value.as_str(), false);
        }
        NodeValue::Boolean(value) => {
            push_scalar(
                output,
                ScalarKind::Boolean,
                position,
                if *value { "true" } else { "false" },
                false,
            );
        }
        NodeValue::Null => push_scalar(output, ScalarKind::Null, position, "null", false),
    }
}

fn push_container_start(
    output: &mut String,
    kind: &str,
    position: NodePosition<'_>,
    depth: usize,
    immediate_child_count: usize,
    scalar_only: bool,
    options: HtmlRenderOptions,
) {
    let label = match position {
        NodePosition::Root => {
            output.push_str("<div data-structured-node=\"");
            push_encoded_attribute(output, kind);
            if immediate_child_count > 0 {
                output.push_str("\" data-structured-group=\"");
                output.push_str(if scalar_only { "scalar-only" } else { "mixed" });
            }
            output.push_str("\">");
            return;
        }
        NodePosition::Child(label) => label,
    };

    output.push_str("<details data-structured-container data-structured-node=\"");
    push_encoded_attribute(output, kind);
    if immediate_child_count > 0 {
        output.push_str("\" data-structured-group=\"");
        output.push_str(if scalar_only { "scalar-only" } else { "mixed" });
    }
    output.push('"');
    if depth <= 2 && immediate_child_count <= options.large_container_threshold() {
        output.push_str(" open");
    }
    output.push_str("><summary>");
    push_label(output, label);
    output.push_str("</summary>");
}

fn is_scalar(node: &NodeValue) -> bool {
    matches!(
        node,
        NodeValue::String(_) | NodeValue::Number(_) | NodeValue::Boolean(_) | NodeValue::Null
    )
}

fn push_scalar(
    output: &mut String,
    kind: ScalarKind,
    position: NodePosition<'_>,
    value: &str,
    empty_string: bool,
) {
    output.push_str("<div data-structured-node=\"");
    push_encoded_attribute(output, kind.attribute_value());
    output.push_str("\">");
    if let NodePosition::Child(label) = position {
        push_label(output, label);
    }
    output.push_str("<span data-structured-value");
    if empty_string {
        output.push_str(" data-structured-empty=\"string\"");
    }
    output.push('>');
    match kind {
        ScalarKind::String => push_rendered_text_with_break_markers(output, value),
        ScalarKind::Number | ScalarKind::Boolean | ScalarKind::Null => {
            push_encoded_text(output, value);
        }
    }
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
            push_rendered_text_with_break_markers(output, key);
        }
        NodeLabel::Index(index) => {
            output.push_str("<span data-structured-label=\"index\">");
            push_encoded_text(output, &index.to_string());
        }
    }
    output.push_str("</span>");
}

fn push_rendered_text_with_break_markers(output: &mut String, value: &str) {
    let mut segment_start = 0;
    let mut characters = value.char_indices().peekable();

    while let Some((offset, character)) = characters.next() {
        if !matches!(character, '\r' | '\n') {
            continue;
        }

        push_encoded_text(output, &value[segment_start..offset]);
        output.push_str("<span data-structured-line-break aria-hidden=\"true\"></span>");

        let mut segment_end = offset + character.len_utf8();
        if character == '\r'
            && let Some(&(line_feed_offset, '\n')) = characters.peek()
        {
            characters.next();
            segment_end = line_feed_offset + '\n'.len_utf8();
        }
        push_encoded_text(output, &value[offset..segment_end]);
        segment_start = segment_end;
    }

    push_encoded_text(output, &value[segment_start..]);
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
