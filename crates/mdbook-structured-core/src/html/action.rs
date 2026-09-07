use super::encode::{push_encoded_attribute, push_encoded_text};

pub(super) enum Action {
    ExpandAll,
    CollapseAll,
}

impl Action {
    pub(super) fn push_html(self, output: &mut String) {
        let (action, label, icon) = match self {
            Self::ExpandAll => (
                "expand-all",
                "Expand all",
                include_str!("icons/expand-all.svg"),
            ),
            Self::CollapseAll => (
                "collapse-all",
                "Collapse all",
                include_str!("icons/collapse-all.svg"),
            ),
        };
        output.push_str("<button type=\"button\" data-structured-action=\"");
        push_encoded_attribute(output, action);
        output.push_str("\" aria-label=\"");
        push_encoded_attribute(output, label);
        output.push_str("\">");
        output.push_str(icon.trim());
        output.push_str("<span data-structured-tooltip aria-hidden=\"true\">");
        push_encoded_text(output, label);
        output.push_str("</span></button>");
    }
}
