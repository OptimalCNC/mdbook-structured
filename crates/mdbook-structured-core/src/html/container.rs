use crate::{MappingEntry, Node, NodeValue, NumberLexeme};

use super::encode::push_encoded_text;
use super::{NodeLabel, ScalarKind, is_scalar, push_label, push_rendered_text_with_break_markers};

#[derive(Clone, Copy)]
pub(super) enum Container<'a> {
    Mapping(&'a [MappingEntry]),
    Sequence(&'a [Node]),
}

impl Container<'_> {
    pub(super) fn kind(self) -> &'static str {
        match self {
            Self::Mapping(_) => "mapping",
            Self::Sequence(_) => "sequence",
        }
    }

    pub(super) fn child_count(self) -> usize {
        match self {
            Self::Mapping(entries) => entries.len(),
            Self::Sequence(items) => items.len(),
        }
    }

    pub(super) fn scalar_only(self) -> bool {
        match self {
            Self::Mapping(entries) => entries.iter().all(|entry| is_scalar(entry.value().value())),
            Self::Sequence(items) => items.iter().all(|item| is_scalar(item.value())),
        }
    }

    pub(super) fn push_summary(self, output: &mut String, label: NodeLabel<'_>) {
        output.push_str("<summary>");
        match label {
            NodeLabel::Key(_) => {
                push_label(output, label);
                if let Self::Sequence(items) = self {
                    output.push(' ');
                    push_count(output, items.len(), "item");
                }
            }
            NodeLabel::Index(_) => {
                output.push_str(
                    "<span class=\"structured-item-heading\"><span data-structured-preview>",
                );
                match self {
                    Self::Mapping(entries) => push_mapping_preview(output, entries),
                    Self::Sequence(items) => push_count(output, items.len(), "item"),
                }
                output.push_str("</span>");
                push_label(output, label);
                output.push_str("</span>");
            }
        }
        output.push_str("</summary>");
    }
}

enum PreviewScalar<'a> {
    String(&'a str),
    Number(&'a NumberLexeme),
    Boolean(bool),
    Null,
}

impl<'a> PreviewScalar<'a> {
    fn parse(value: &'a NodeValue) -> Option<Self> {
        match value {
            NodeValue::String(value) => Some(Self::String(value)),
            NodeValue::Number(value) => Some(Self::Number(value)),
            NodeValue::Boolean(value) => Some(Self::Boolean(*value)),
            NodeValue::Null => Some(Self::Null),
            NodeValue::Mapping(_) | NodeValue::Sequence(_) => None,
        }
    }

    fn display(self) -> (ScalarKind, &'a str) {
        match self {
            Self::String(value) => (ScalarKind::String, value),
            Self::Number(value) => (ScalarKind::Number, value.as_str()),
            Self::Boolean(value) => (ScalarKind::Boolean, if value { "true" } else { "false" }),
            Self::Null => (ScalarKind::Null, "null"),
        }
    }
}

fn push_mapping_preview(output: &mut String, entries: &[MappingEntry]) {
    let fields = entries.iter().filter_map(|entry| {
        PreviewScalar::parse(entry.value().value()).map(|value| (entry.decoded_key(), value))
    });
    let mut shown = 0;
    for (key, value) in fields.take(2) {
        output.push_str("<span data-structured-preview-field><span data-structured-preview-key");
        push_preview_text(output, key);
        output.push_str(": <span data-structured-preview-value=\"");
        let (kind, text) = value.display();
        output.push_str(kind.attribute_value());
        output.push('"');
        push_preview_text(output, text);
        output.push_str("</span>");
        shown += 1;
    }
    if shown == 0 {
        push_count(output, entries.len(), "field");
    } else if shown < entries.len() {
        output.push_str("<span data-structured-preview-more aria-label=\"More fields\">…</span>");
    }
}

fn push_preview_text(output: &mut String, value: &str) {
    if value.is_empty() {
        output.push_str(" data-structured-preview-empty");
    }
    output.push('>');
    push_rendered_text_with_break_markers(output, value);
    output.push_str("</span>");
}

fn push_count(output: &mut String, count: usize, noun: &str) {
    output.push_str("<span data-structured-count>");
    push_encoded_text(output, &count.to_string());
    output.push(' ');
    push_encoded_text(output, noun);
    if count != 1 {
        output.push('s');
    }
    output.push_str("</span>");
}
